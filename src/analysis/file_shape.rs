//! File Shape Tool
//!
//! Extracts the high-level structure of a source file (functions, classes, imports)
//! without the implementation details.
//!
//! **DEPRECATED**: This module is deprecated. Use `view_code` module instead.
//! `file_shape` only supports Rust/Swift/Python, while `view_code` supports all 9 languages.

#![allow(deprecated)]

use crate::analysis::dependencies::{
    find_js_ts_dependencies, find_python_dependencies, find_rust_dependencies,
};
use crate::parser::{detect_language, parse_code, Language};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tree_sitter::{Query, QueryCursor, Tree};

#[allow(dead_code)]
const MAX_TEMPLATE_DEPTH: usize = 50;

#[derive(Debug, serde::Serialize)]
pub struct FunctionInfo {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct StructInfo {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct ClassInfo {
    pub name: String,
    pub line: usize,
}

#[allow(dead_code)]
#[derive(Debug, serde::Serialize)]
pub struct FileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub functions: Vec<FunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<StructInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<FileShape>,
}

#[deprecated(
    since = "0.2.0",
    note = "Use shape::extract_enhanced_shape instead. extract_shape only supports Rust/Swift/Python, while extract_enhanced_shape supports all 9 languages."
)]
pub fn extract_shape(
    tree: &Tree,
    source: &str,
    language: Language,
) -> Result<FileShape, io::Error> {
    match language {
        Language::Rust => extract_rust_shape(tree, source),
        Language::Swift => extract_swift_shape(tree, source),
        Language::Python => extract_python_shape(tree, source),
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("extract_shape not implemented for {}", language.name()),
        )),
    }
}

fn extract_python_shape(tree: &Tree, source: &str) -> Result<FileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_python::LANGUAGE.into(),
        r#"
        (function_definition name: (identifier) @func.name) @func
        (class_definition name: (identifier) @class.name) @class
        (import_statement) @import
        (import_from_statement) @import
        "#,
    )
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in function name: {e}"),
                        )
                    })?;
                    functions.push(FunctionInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "class.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in class name: {e}"),
                        )
                    })?;
                    classes.push(ClassInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "import" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in import statement: {e}"),
                        )
                    })?;
                    imports.push(text.to_string());
                }
                _ => {}
            }
        }
    }

    Ok(FileShape {
        path: None,
        functions,
        structs: vec![],
        classes,
        imports,
        dependencies: vec![],
    })
}

fn extract_rust_shape(tree: &Tree, source: &str) -> Result<FileShape, io::Error> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"
        (function_item name: (identifier) @func.name) @func
        (struct_item name: (type_identifier) @struct.name) @struct
        (enum_item name: (type_identifier) @enum.name) @enum
        (trait_item name: (type_identifier) @trait.name) @trait
        (impl_item trait: (type_identifier)? @impl.trait type: (type_identifier) @impl.type) @impl
        (use_declaration) @use
        "#,
    )
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in function name: {e}"),
                        )
                    })?;
                    functions.push(FunctionInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "struct.name" | "enum.name" | "trait.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in struct/enum/trait name: {e}"),
                        )
                    })?;
                    structs.push(StructInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "use" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in use statement: {e}"),
                        )
                    })?;
                    imports.push(text.to_string());
                }
                _ => {}
            }
        }
    }

    Ok(FileShape {
        path: None,
        functions,
        structs,
        classes: vec![],
        imports,
        dependencies: vec![],
    })
}

#[allow(dead_code)]
fn extract_swift_shape(tree: &Tree, source: &str) -> Result<FileShape, io::Error> {
    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_swift::LANGUAGE.into(),
        r#"
        (function_declaration (simple_identifier) @func.name) @func
        (class_declaration (type_identifier) @class.name) @class
        (protocol_declaration (type_identifier) @protocol.name) @protocol
        (import_declaration) @import
        "#,
    )
    .map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to create tree-sitter query: {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in function name: {e}"),
                        )
                    })?;
                    functions.push(FunctionInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }

                "class.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in class name: {e}"),
                        )
                    })?;

                    // In Swift, both struct and class use class_declaration node type
                    // Search for "struct" or "class" keyword among the children
                    let is_struct = if let Some(parent) = node.parent() {
                        let mut found_struct = false;
                        for i in 0..parent.child_count() {
                            if let Some(child) = parent.child(i) {
                                if let Ok(keyword) = child.utf8_text(source.as_bytes()) {
                                    if keyword == "struct" {
                                        found_struct = true;
                                        break;
                                    } else if keyword == "class" {
                                        break;
                                    }
                                }
                            }
                        }
                        found_struct
                    } else {
                        false
                    };

                    if is_struct {
                        structs.push(StructInfo {
                            name: text.to_string(),
                            line: node.start_position().row + 1,
                        });
                    } else {
                        classes.push(ClassInfo {
                            name: text.to_string(),
                            line: node.start_position().row + 1,
                        });
                    }
                }
                "protocol.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in protocol name: {e}"),
                        )
                    })?;
                    // Treat protocols as structs (they are like interfaces)
                    structs.push(StructInfo {
                        name: text.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "import" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in import: {e}"),
                        )
                    })?;
                    imports.push(text.to_string());
                }
                _ => {}
            }
        }
    }

    Ok(FileShape {
        path: None,
        functions,
        structs,
        classes,
        imports,
        dependencies: vec![],
    })
}

#[allow(dead_code)]
#[deprecated(
    since = "0.2.0",
    note = "Internal function - use view_code module instead for multi-language support"
)]
fn build_shape_tree(
    path: &Path,
    project_root: &Path,
    include_deps: bool,
    visited: &mut HashSet<PathBuf>,
) -> Result<FileShape, io::Error> {
    // Avoid infinite recursion in case of cyclic module structures
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        // Already processed – just return the flat shape
        let source = fs::read_to_string(path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to read file {}: {e}", path.display()),
            )
        })?;
        let language = detect_language(path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                format!("Cannot detect language for file {}: {e}", path.display()),
            )
        })?;
        let tree = parse_code(&source, language).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse {} code: {e}", language.name()),
            )
        })?;
        let mut shape = extract_shape(&tree, &source, language)?;
        shape.path = Some(crate::analysis::path_utils::to_relative_path(
            &path.to_string_lossy(),
        ));
        return Ok(shape);
    }
    visited.insert(canonical);

    let source = fs::read_to_string(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file {}: {e}", path.display()),
        )
    })?;

    let language = detect_language(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file {}: {e}", path.display()),
        )
    })?;
    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let mut shape = extract_shape(&tree, &source, language)?;
    shape.path = Some(crate::analysis::path_utils::to_relative_path(
        &path.to_string_lossy(),
    ));

    if include_deps {
        let mut deps = Vec::new();

        match language {
            Language::Rust => {
                for dep_path in find_rust_dependencies(&source, path, project_root) {
                    let dep_shape =
                        build_shape_tree(&dep_path, project_root, include_deps, visited)?;
                    deps.push(dep_shape);
                }
            }
            Language::Python => {
                for dep_path in find_python_dependencies(&source, path, project_root) {
                    let dep_shape =
                        build_shape_tree(&dep_path, project_root, include_deps, visited)?;
                    deps.push(dep_shape);
                }
            }
            Language::JavaScript | Language::TypeScript => {
                for dep_path in find_js_ts_dependencies(&source, path, project_root) {
                    let dep_shape =
                        build_shape_tree(&dep_path, project_root, include_deps, visited)?;
                    deps.push(dep_shape);
                }
            }
            _ => {
                // Dependency expansion is not implemented for other languages.
            }
        }

        shape.dependencies = deps;
    }

    Ok(shape)
}

/// Find the project root by walking up to the nearest directory containing Cargo.toml.
#[allow(dead_code)]
fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.is_file() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}

// ============================================================================
// Template Support (Askama/Jinja2)
// ============================================================================

use regex::Regex;

/// Template dependency info
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, Clone)]
pub struct TemplateDependency {
    pub path: String,
    pub dependency_type: String, // "extends" or "include"
    pub name: String,
}

/// Template file shape (when merge_templates=true)
#[allow(dead_code)]
#[derive(Debug, serde::Serialize)]
pub struct MergedTemplateShape {
    pub path: String,
    pub merged_content: String,
    pub dependencies: Vec<TemplateDependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_structs: Option<Vec<crate::analysis::askama::TemplateStructInfo>>,
}

/// Find template dependencies (extends/includes) in a template file
///
/// Returns a list of template dependencies with their types and paths.
#[allow(dead_code)]
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
#[allow(dead_code)]
fn merge_template(
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

    let source = fs::read_to_string(template_path)?;

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
#[allow(dead_code)]
fn extract_blocks(source: &str) -> Result<std::collections::HashMap<String, String>, io::Error> {
    let mut blocks = std::collections::HashMap::new();

    let block_re = Regex::new(r#"\{%\s*block\s+(\w+)\s*%\}(.*?)\{%\s*endblock\s*%\}"#).unwrap();

    for cap in block_re.captures_iter(source) {
        let block_name = cap[1].to_string();
        let block_content = cap[2].to_string();
        blocks.insert(block_name, block_content);
    }

    Ok(blocks)
}

/// Replace {% block name %}...{% endblock %} sections in a template with provided blocks
#[allow(dead_code)]
fn replace_blocks(
    template: &str,
    blocks: &std::collections::HashMap<String, String>,
) -> Result<String, io::Error> {
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
