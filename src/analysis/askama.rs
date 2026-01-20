use eyre::{Result, WrapErr};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use tree_sitter::Node;

use crate::analysis::path_utils;
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{parse_code, Language};

const MAX_NESTED_DEPTH: u8 = 3;
const MAX_TEMPLATE_DEPTH: usize = 50;

/// MCP tool execute function for template_context
///
/// Compact output schema:
/// - `tpl`: template path (relative)
/// - `h`: header for `ctx` rows
/// - `ctx`: newline-delimited rows (pipe-delimited fields)
/// - `sh`: header for `s` rows
/// - `s`: struct definition locations (struct|file|line)
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let template_path_str = arguments["template_path"]
        .as_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "template_path is required"))?;

    let cwd = std::env::current_dir()
        .map_err(|e| io::Error::other(format!("Failed to get current directory: {e}")))?;

    let mut template_path = PathBuf::from(template_path_str);
    if template_path.is_relative() {
        template_path = cwd.join(template_path);
    }

    // Askama templates typically live under `<project>/templates/...`.
    // Our fixture projects don't have a Cargo.toml, so infer the project root
    // from the templates directory when possible.
    let project_root = find_templates_dir(template_path.parent().unwrap())
        .and_then(|templates_dir| templates_dir.parent().map(|p| p.to_path_buf()))
        .or_else(|| path_utils::find_project_root(&template_path))
        .or_else(|| template_path.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot determine project root"))?;

    let structs = find_askama_structs_for_template(&template_path, &project_root)
        .map_err(|e| io::Error::other(format!("Failed to find template structs: {e}")))?;

    let tpl_rel = path_utils::to_relative_path(template_path.to_string_lossy().as_ref());

    let ctx_header = "struct|field|type";
    let ctx_rows = template_structs_to_rows(&structs);

    let struct_header = "struct|file|line";
    let struct_rows = template_struct_locations_to_rows(&structs);

    let output = serde_json::json!({
        "tpl": tpl_rel,
        "h": ctx_header,
        "ctx": ctx_rows,
        "sh": struct_header,
        "s": struct_rows
    });

    let json_string = serde_json::to_string(&output)
        .map_err(|e| io::Error::other(format!("Failed to serialize output: {e}")))?;

    Ok(CallToolResult::success(json_string))
}

fn template_structs_to_rows(structs: &[TemplateStructInfo]) -> String {
    let mut rows = Vec::new();

    for s in structs {
        append_field_rows(&mut rows, &s.struct_name, &s.fields);
    }

    rows.join("\n")
}

fn append_field_rows(rows: &mut Vec<String>, struct_name: &str, fields: &[TemplateField]) {
    for field in fields {
        let row =
            format::format_row(&[struct_name, field.name.as_str(), field.field_type.as_str()]);
        rows.push(row);

        if let Some(nested) = &field.nested_definition {
            append_field_rows(rows, nested.type_name.as_str(), &nested.fields);
        }
    }
}

fn template_struct_locations_to_rows(structs: &[TemplateStructInfo]) -> String {
    structs
        .iter()
        .map(|s| {
            let file = path_utils::to_relative_path(s.file_path.to_string_lossy().as_ref());
            let line = s.line.to_string();
            format::format_row(&[s.struct_name.as_str(), file.as_str(), line.as_str()])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Information about a struct that serves as a template context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateStructInfo {
    /// Name of the struct (e.g., "DashboardTemplate")
    pub struct_name: String,

    /// All fields with their types (resolved up to 3 levels)
    pub fields: Vec<TemplateField>,

    /// File where the struct is defined
    pub file_path: PathBuf,

    /// Line number of struct definition
    pub line: usize,
}

/// A field in a template struct
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TemplateField {
    /// Field name (e.g., "user_name")
    pub name: String,

    /// Field type as string (e.g., "String", "Vec<Item>")
    pub field_type: String,

    /// Nested type definition (if resolved, up to 3 levels deep)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nested_definition: Option<Box<NestedTypeDefinition>>,
}

/// Resolved definition of a nested type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NestedTypeDefinition {
    /// Type name (e.g., "Statistics")
    pub type_name: String,

    /// Fields of this type
    pub fields: Vec<TemplateField>,

    /// Depth level (1, 2, or 3)
    pub depth: u8,
}

/// Internal struct for matching templates during search
#[derive(Debug)]
struct TemplateStructMatch {
    struct_name: String,
    fields: Vec<TemplateField>,
    file_path: PathBuf,
    line: usize,
}

/// Main entry point: Find all Rust structs associated with a template file
///
/// # Arguments
/// * `template_path` - Absolute path to the template file
/// * `project_root` - Root directory of the project to search
pub fn find_askama_structs_for_template(
    template_path: &Path,
    project_root: &Path,
) -> Result<Vec<TemplateStructInfo>> {
    // Find templates directory
    let templates_dir = find_templates_dir(template_path.parent().unwrap())
        .ok_or_else(|| eyre::eyre!("Could not find templates directory"))?;

    // Calculate relative template path
    let relative_path = normalize_template_path(template_path, &templates_dir)?;

    // Search all Rust files in project
    let matches = search_rust_files_for_template(&relative_path, project_root)?;

    // Convert matches to TemplateStructInfo
    let results = matches
        .into_iter()
        .map(|m| TemplateStructInfo {
            struct_name: m.struct_name,
            fields: m.fields,
            file_path: m.file_path,
            line: m.line,
        })
        .collect();

    Ok(results)
}

/// Normalize template path to canonical form for matching
fn normalize_template_path(template_path: &Path, templates_dir: &Path) -> Result<String> {
    let relative = template_path
        .strip_prefix(templates_dir)
        .wrap_err("Template path not under templates directory")?;

    Ok(relative.to_string_lossy().to_string())
}

/// Search all Rust files in the project for template attributes
fn search_rust_files_for_template(
    target_template_path: &str,
    project_root: &Path,
) -> Result<Vec<TemplateStructMatch>> {
    let mut matches = Vec::new();

    // Walk all Rust files in project
    for entry in walkdir::WalkDir::new(project_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(mut file_matches) =
                extract_template_structs_from_file(path, target_template_path)
            {
                matches.append(&mut file_matches);
            }
        }
    }

    Ok(matches)
}

/// Extract template struct definitions from a single Rust file
fn extract_template_structs_from_file(
    file_path: &Path,
    target_template_path: &str,
) -> Result<Vec<TemplateStructMatch>> {
    let source_code = std::fs::read_to_string(file_path)?;
    let tree = parse_code(&source_code, Language::Rust)?;

    let root = tree.root_node();
    let mut matches = Vec::new();

    // Get project root - walk up to find Cargo.toml
    let project_root =
        find_project_root(file_path).unwrap_or_else(|| file_path.parent().unwrap().to_path_buf());

    // Find all struct items
    find_template_structs_recursive(
        root,
        &source_code,
        target_template_path,
        file_path,
        &mut matches,
        &project_root,
    )?;

    Ok(matches)
}

/// Recursively find structs with matching template attributes
fn find_template_structs_recursive(
    node: Node,
    source_code: &str,
    target_template_path: &str,
    file_path: &Path,
    matches: &mut Vec<TemplateStructMatch>,
    project_root: &Path,
) -> Result<()> {
    if node.kind() == "struct_item" {
        // Check if this struct has the right attributes
        if let Some(template_path) = check_struct_for_template_attribute(node, source_code)? {
            log::debug!(
                "Found template path: '{}', target: '{}'",
                template_path,
                target_template_path
            );
            if template_path == target_template_path {
                // Extract struct name
                if let Some(name_node) = node.child_by_field_name("name") {
                    let struct_name = name_node.utf8_text(source_code.as_bytes())?;
                    let line = node.start_position().row + 1;

                    // Extract fields immediately while we have the node
                    let fields =
                        extract_struct_fields(node, source_code, file_path, project_root, 0)?;

                    matches.push(TemplateStructMatch {
                        struct_name: struct_name.to_string(),
                        fields,
                        file_path: file_path.to_path_buf(),
                        line,
                    });
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_template_structs_recursive(
            child,
            source_code,
            target_template_path,
            file_path,
            matches,
            project_root,
        )?;
    }

    Ok(())
}

/// Check if a struct has derive(Template) and template(path = "...") attributes
fn check_struct_for_template_attribute(
    struct_node: Node,
    source_code: &str,
) -> Result<Option<String>> {
    let mut has_derive_template = false;
    let mut template_path = None;

    // Look for attribute items before the struct
    // Stop when we hit a non-attribute/non-comment node to avoid reading other structs' attributes
    let mut sibling = struct_node.prev_sibling();
    while let Some(node) = sibling {
        match node.kind() {
            "attribute_item" => {
                let attr_text = node.utf8_text(source_code.as_bytes())?;

                // Check for derive(Template)
                if attr_text.contains("derive") && attr_text.contains("Template") {
                    has_derive_template = true;
                }

                // Check for template(path = "...")
                if attr_text.contains("template") {
                    if let Some(path) = extract_template_path_from_attribute(node, source_code) {
                        template_path = Some(path);
                    }
                }
            }
            "line_comment" | "block_comment" => {
                // Skip comments, continue looking
            }
            _ => {
                // Hit a non-attribute, non-comment node - stop looking
                break;
            }
        }
        sibling = node.prev_sibling();
    }

    if has_derive_template && template_path.is_some() {
        Ok(template_path)
    } else {
        Ok(None)
    }
}

/// Extract template path from attribute: #[template(path = "admin/dashboard.html")]
fn extract_template_path_from_attribute(attribute_node: Node, source_code: &str) -> Option<String> {
    let text = attribute_node.utf8_text(source_code.as_bytes()).ok()?;

    // Simple regex-like parsing for path = "..."
    if let Some(start) = text.find("path") {
        let after_path = &text[start..];
        if let Some(quote_start) = after_path.find('"') {
            let after_quote = &after_path[quote_start + 1..];
            if let Some(quote_end) = after_quote.find('"') {
                return Some(after_quote[..quote_end].to_string());
            }
        }
    }

    None
}

/// Extract all fields from a struct definition node
fn extract_struct_fields(
    struct_node: Node,
    source_code: &str,
    _file_path: &Path,
    project_root: &Path,
    current_depth: u8,
) -> Result<Vec<TemplateField>> {
    let mut fields = Vec::new();

    // Find the field_declaration_list
    if let Some(body) = struct_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                // Extract field name
                let name = if let Some(name_node) = child.child_by_field_name("name") {
                    name_node.utf8_text(source_code.as_bytes())?.to_string()
                } else {
                    continue;
                };

                // Extract field type
                let field_type = if let Some(type_node) = child.child_by_field_name("type") {
                    type_node.utf8_text(source_code.as_bytes())?.to_string()
                } else {
                    continue;
                };

                // Resolve nested type if we haven't reached max depth
                let nested_definition = if current_depth < MAX_NESTED_DEPTH {
                    resolve_type_definition(&field_type, project_root, current_depth + 1)
                } else {
                    None
                };

                fields.push(TemplateField {
                    name,
                    field_type,
                    nested_definition,
                });
            }
        }
    }

    Ok(fields)
}

/// Resolve a type definition to its fields (up to MAX_NESTED_DEPTH levels)
fn resolve_type_definition(
    type_name: &str,
    project_root: &Path,
    current_depth: u8,
) -> Option<Box<NestedTypeDefinition>> {
    if current_depth > MAX_NESTED_DEPTH {
        return None;
    }

    // Extract base type name from complex types like Vec<T>, Option<T>, etc.
    let base_type = extract_base_type_name(type_name);

    // Skip primitive types and standard library types
    if is_primitive_or_std_type(&base_type) {
        return None;
    }

    // Search for the type definition in the project
    if let Ok(type_def) = find_type_definition(&base_type, project_root, current_depth) {
        Some(Box::new(type_def))
    } else {
        None
    }
}

/// Find the templates directory by walking up the file system
///
/// Searches for a directory named "templates" starting from the given path,
/// walking up the directory tree. Returns the path to the templates directory.
pub fn find_templates_dir(file_path: &Path) -> Option<PathBuf> {
    let mut current = file_path.parent()?;
    let mut depth = 0;
    const MAX_DEPTH: usize = 10;

    while depth < MAX_DEPTH {
        // Check if current dir is named "templates"
        if current
            .file_name()
            .map(|n| n == "templates")
            .unwrap_or(false)
        {
            return Some(current.to_path_buf());
        }

        // Check if "templates" subdir exists
        let templates_subdir = current.join("templates");
        if templates_subdir.is_dir() {
            return Some(templates_subdir);
        }

        current = current.parent()?;
        depth += 1;
    }

    None
}

/// Extract base type name from complex types
fn extract_base_type_name(type_str: &str) -> String {
    // Handle Vec<T>, Option<T>, HashMap<K,V>, etc.
    if let Some(start) = type_str.find('<') {
        type_str[..start].trim().to_string()
    } else {
        type_str.trim().to_string()
    }
}

/// Check if a type is a primitive or standard library type
fn is_primitive_or_std_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "String"
            | "Vec"
            | "Option"
            | "Result"
            | "HashMap"
            | "HashSet"
            | "Box"
            | "Rc"
            | "Arc"
            | "Cell"
            | "RefCell"
    )
}

/// Find project root by walking up to find Cargo.toml
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    while let Some(parent) = current.parent() {
        if parent.join("Cargo.toml").exists() {
            return Some(parent.to_path_buf());
        }
        current = parent;
    }
    None
}

/// Find a type definition in the project
fn find_type_definition(
    type_name: &str,
    project_root: &Path,
    current_depth: u8,
) -> Result<NestedTypeDefinition> {
    // Walk all Rust files looking for struct/enum with this name
    for entry in walkdir::WalkDir::new(project_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            if let Ok(Some(type_def)) =
                find_type_in_file(path, type_name, project_root, current_depth)
            {
                return Ok(type_def);
            }
        }
    }

    Err(eyre::eyre!("Type {} not found in project", type_name))
}

/// Find a type definition in a specific file
fn find_type_in_file(
    file_path: &Path,
    type_name: &str,
    project_root: &Path,
    current_depth: u8,
) -> Result<Option<NestedTypeDefinition>> {
    let source_code = std::fs::read_to_string(file_path)?;
    let tree = parse_code(&source_code, Language::Rust)?;

    let root = tree.root_node();

    // Search for struct_item with matching name
    let result = find_struct_by_name(root, type_name, &source_code, project_root, current_depth)?;

    Ok(result)
}

/// Recursively find a struct by name
fn find_struct_by_name(
    node: Node,
    type_name: &str,
    source_code: &str,
    project_root: &Path,
    current_depth: u8,
) -> Result<Option<NestedTypeDefinition>> {
    if node.kind() == "struct_item" {
        if let Some(name_node) = node.child_by_field_name("name") {
            let struct_name = name_node.utf8_text(source_code.as_bytes())?;
            if struct_name == type_name {
                // Found it! Extract fields
                let fields = extract_struct_fields(
                    node,
                    source_code,
                    Path::new(""),
                    project_root,
                    current_depth,
                )?;

                return Ok(Some(NestedTypeDefinition {
                    type_name: type_name.to_string(),
                    fields,
                    depth: current_depth,
                }));
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(result) =
            find_struct_by_name(child, type_name, source_code, project_root, current_depth)?
        {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

// ============================================================================
// Template Support (Askama/Jinja2)
// ============================================================================

/// Template dependency info
#[derive(Debug, serde::Serialize, Clone)]
pub struct TemplateDependency {
    pub path: String,
    pub dependency_type: String, // "extends" or "include"
    pub name: String,
}

/// Template file shape (when merge_templates=true)
#[derive(Debug, serde::Serialize)]
pub struct MergedTemplateShape {
    pub path: String,
    pub merged_content: String,
    pub dependencies: Vec<TemplateDependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_structs: Option<Vec<TemplateStructInfo>>,
}

/// Find template dependencies (extends/includes) in a template file
///
/// Returns a list of template dependencies with their types and paths.
pub fn find_template_dependencies(
    source: &str,
    templates_dir: &Path,
) -> Result<Vec<TemplateDependency>, io::Error> {
    let mut dependencies = Vec::new();

    // Regex for {% extends "base.html" %}
    let extends_re = Regex::new(r#"\{%\s*extends\s+["']([^"']+)["']\s*%\}"#).unwrap();
    // Regex for {% include "partial.html" %}
    let include_re = Regex::new(r#"\{%\s*include\s+["']([^"']+)["']\s*%\}"#).unwrap();

    // Find extends
    for cap in extends_re.captures_iter(source) {
        let template_name = &cap[1];
        let template_path = templates_dir.join(template_name);
        // Only include if the template file exists
        if template_path.exists() {
            dependencies.push(TemplateDependency {
                path: template_name.to_string(),
                dependency_type: "extends".to_string(),
                name: template_name.to_string(),
            });
        }
    }

    // Find includes
    for cap in include_re.captures_iter(source) {
        let template_name = &cap[1];
        let template_path = templates_dir.join(template_name);
        // Only include if the template file exists
        if template_path.exists() {
            dependencies.push(TemplateDependency {
                path: template_name.to_string(),
                dependency_type: "include".to_string(),
                name: template_name.to_string(),
            });
        }
    }

    Ok(dependencies)
}

/// Recursively merge a template with its parent templates and includes
///
/// Handles {% extends %} and {% include %} directives, merging content appropriately.
pub fn merge_template(
    template_path: &Path,
    templates_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    recursion_stack: &mut Vec<PathBuf>,
) -> Result<String, io::Error> {
    // Check for circular dependencies
    if recursion_stack.contains(&template_path.to_path_buf()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Circular template dependency detected: {}",
                template_path.display()
            ),
        ));
    }

    // Check recursion depth
    if recursion_stack.len() >= MAX_TEMPLATE_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Template recursion depth exceeded (max: {MAX_TEMPLATE_DEPTH})"),
        ));
    }

    recursion_stack.push(template_path.to_path_buf());
    visited.insert(template_path.to_path_buf());

    let source = std::fs::read_to_string(template_path)?;

    // Check for {% extends "parent.html" %}
    let extends_re = Regex::new(r#"\{%\s*extends\s+["']([^"']+)["']\s*%\}"#).unwrap();
    if let Some(cap) = extends_re.captures(&source) {
        let parent_name = &cap[1];
        let parent_path = templates_dir.join(parent_name);

        // Recursively merge parent
        let parent_content = merge_template(&parent_path, templates_dir, visited, recursion_stack)?;

        // Extract blocks from current template
        let blocks = extract_blocks(&source)?;

        // Replace blocks in parent
        let merged = replace_blocks(&parent_content, &blocks)?;

        recursion_stack.pop();
        return Ok(merged);
    }

    // Handle {% include "partial.html" %}
    let include_re = Regex::new(r#"\{%\s*include\s+["']([^"']+)["']\s*%\}"#).unwrap();
    let mut result = source.clone();

    for cap in include_re.captures_iter(&source) {
        let include_name = &cap[1];
        let include_path = templates_dir.join(include_name);

        let include_content =
            merge_template(&include_path, templates_dir, visited, recursion_stack)?;

        // Replace the include directive with the content
        let directive = &cap[0];
        result = result.replace(directive, &include_content);
    }

    recursion_stack.pop();
    Ok(result)
}

/// Extract {% block name %}...{% endblock %} sections from a template
fn extract_blocks(source: &str) -> Result<HashMap<String, String>, io::Error> {
    let mut blocks = HashMap::new();

    let block_re = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}(.*?)\{%\s*endblock\s*%\}"#).unwrap();

    for cap in block_re.captures_iter(source) {
        let block_name = cap[1].to_string();
        let block_content = cap[2].to_string();
        blocks.insert(block_name, block_content);
    }

    Ok(blocks)
}

/// Replace {% block name %}...{% endblock %} sections in a template with provided blocks
fn replace_blocks(template: &str, blocks: &HashMap<String, String>) -> Result<String, io::Error> {
    let block_re = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}.*?\{%\s*endblock\s*%\}"#).unwrap();

    let mut result = template.to_string();

    for cap in block_re.captures_iter(template) {
        let block_name = &cap[1];
        if let Some(replacement) = blocks.get(block_name) {
            let full_block = &cap[0];
            result = result.replace(full_block, replacement);
        }
    }

    Ok(result)
}
