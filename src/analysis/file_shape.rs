//! File Shape Tool
//!
//! Extracts the high-level structure of a source file (functions, classes, imports)
//! without the implementation details.

use crate::analysis::path_utils;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tree_sitter::{Query, QueryCursor, Tree};

const MAX_TEMPLATE_DEPTH: usize = 50;

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

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path_str = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);
    let merge_templates = arguments["merge_templates"].as_bool().unwrap_or(false);

    log::info!("Extracting shape of file: {file_path_str} (include_deps: {include_deps}, merge_templates: {merge_templates})");

    let path = Path::new(file_path_str);

    // Handle template merging if requested
    if merge_templates {
        // Validate this is a template file
        let templates_dir = find_templates_dir(path).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "merge_templates=true requires file to be in a 'templates/' directory",
            )
        })?;

        let source = fs::read_to_string(path)?;
        let mut visited = HashSet::new();
        let mut recursion_stack = Vec::new();

        let merged_content =
            merge_template(path, &templates_dir, &mut visited, &mut recursion_stack)?;
        let dependencies = find_template_dependencies(&source, &templates_dir)?;

        let merged_shape = MergedTemplateShape {
            path: path.to_string_lossy().to_string(),
            merged_content,
            dependencies,
        };

        let shape_json = serde_json::to_string(&merged_shape).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize merged template: {e}"),
            )
        })?;

        return Ok(CallToolResult::success(shape_json));
    }

    // Detect language to check if it's HTML or CSS
    let language = detect_language(path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to detect language: {e}"),
        )
    })?;

    // Handle HTML and CSS specially
    match language {
        Language::Html => {
            let source = fs::read_to_string(path)?;
            let tree = parse_code(&source, language).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse HTML: {e}"),
                )
            })?;
            let html_shape =
                crate::analysis::shape::extract_html_shape(&tree, &source, Some(file_path_str))?;
            let shape_json = serde_json::to_string(&html_shape).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to serialize HTML shape to JSON: {e}"),
                )
            })?;
            return Ok(CallToolResult::success(shape_json));
        }
        Language::Css => {
            let source = fs::read_to_string(path)?;
            let css_shape =
                crate::analysis::shape::extract_css_tailwind(&source, Some(file_path_str))?;
            let shape_json = serde_json::to_string(&css_shape).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to serialize CSS shape to JSON: {e}"),
                )
            })?;
            return Ok(CallToolResult::success(shape_json));
        }
        _ => {
            // Handle other languages normally
        }
    }

    // Determine project root (directory containing Cargo.toml if present)
    let project_root = find_project_root(path).unwrap_or_else(|| {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });

    let mut visited = HashSet::new();
    let mut shape = build_shape_tree(path, &project_root, include_deps, &mut visited)?;

    // Convert path to relative path before serializing
    if let Some(ref path_str) = shape.path {
        shape.path = Some(path_utils::to_relative_path(path_str));
    }

    let shape_json = serde_json::to_string(&shape).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize shape to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(shape_json))
}

pub fn extract_shape(
    tree: &Tree,
    source: &str,
    language: Language,
) -> Result<FileShape, io::Error> {
    match language {
        Language::Rust => extract_rust_shape(tree, source),
        Language::Python => extract_python_shape(tree, source),
        Language::JavaScript => extract_js_shape(tree, source),
        Language::TypeScript => extract_ts_shape(tree, source),
        Language::Html | Language::Css => {
            // HTML and CSS don't fit the FileShape model
            // Return empty shape - they are handled separately in execute()
            Ok(FileShape {
                path: None,
                functions: vec![],
                structs: vec![],
                classes: vec![],
                imports: vec![],
                dependencies: vec![],
            })
        }
    }
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
        (use_declaration) @import
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
                "struct.name" => {
                    let text = node.utf8_text(source.as_bytes()).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Invalid UTF-8 in struct name: {e}"),
                        )
                    })?;
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
        classes: vec![],
        imports,
        dependencies: vec![],
    })
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
        structs: vec![],
        classes,
        imports,
        dependencies: vec![],
    })
}

fn extract_js_shape(tree: &Tree, source: &str) -> Result<FileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_javascript::LANGUAGE.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (class_declaration name: (identifier) @class.name) @class
        (import_statement) @import
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
        structs: vec![],
        classes,
        imports,
        dependencies: vec![],
    })
}

fn extract_ts_shape(tree: &Tree, source: &str) -> Result<FileShape, io::Error> {
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut imports = Vec::new();

    let query = Query::new(
        &tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        r#"
        (function_declaration name: (identifier) @func.name) @func
        (class_declaration name: (type_identifier) @class.name) @class
        (import_statement) @import
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
        structs: vec![],
        classes,
        imports,
        dependencies: vec![],
    })
}

/// Build a file shape (and optionally its dependency tree) starting from a path.
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
        shape.path = Some(path.to_string_lossy().to_string());
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
    shape.path = Some(path.to_string_lossy().to_string());

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

/// For Rust files, find module dependencies that live in this project.
///
/// Uses tree-sitter to parse `mod foo;` or `pub mod foo;` declarations (not inline modules)
/// and resolves them to `foo.rs` or `foo/mod.rs` under the same directory, constrained to
/// `project_root` so that only project files are included.
pub fn find_rust_dependencies(source: &str, file_path: &Path, project_root: &Path) -> Vec<PathBuf> {
    let mut deps = Vec::new();

    let dir = file_path.parent().unwrap_or(project_root);

    // Parse the source with tree-sitter
    let language = tree_sitter_rust::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        log::warn!("Failed to set Rust language for parser");
        return deps;
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => {
            log::warn!("Failed to parse Rust source for module dependencies");
            return deps;
        }
    };

    // Query for mod declarations (excluding inline modules with bodies)
    // We want: `mod foo;` or `pub mod foo;`
    // We don't want: `mod foo { ... }`
    let query_str = r#"
        (mod_item
            name: (identifier) @mod_name
            !body
        )
    "#;

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(e) => {
            log::warn!("Failed to create Rust mod query: {e}");
            return deps;
        }
    };

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            if let Ok(mod_name) = capture.node.utf8_text(source.as_bytes()) {
                // Try foo.rs
                let candidate = dir.join(format!("{mod_name}.rs"));
                if candidate.is_file() && candidate.starts_with(project_root) {
                    deps.push(candidate);
                    continue;
                }

                // Try foo/mod.rs
                let candidate = dir.join(mod_name).join("mod.rs");
                if candidate.is_file() && candidate.starts_with(project_root) {
                    deps.push(candidate);
                }
            }
        }
    }

    deps
}

/// For Python files, find import dependencies that live in this project.
///
/// Parses `import foo` and `from foo import bar` statements and resolves them to
/// `foo.py` or `foo/__init__.py` under the project root.
pub fn find_python_dependencies(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    let mut deps = Vec::new();
    let dir = file_path.parent().unwrap_or(project_root);

    let language = tree_sitter_python::LANGUAGE.into();
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return deps;
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return deps,
    };

    // Query for import statements
    let query_str = r#"
        (import_statement
            name: (dotted_name) @import_name
        )
        (import_from_statement
            module_name: (dotted_name) @import_name
        )
    "#;

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(_) => return deps,
    };

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            if let Ok(module) = capture.node.utf8_text(source.as_bytes()) {
                push_python_module(&mut deps, module, dir, project_root);
            }
        }
    }

    deps
}

fn push_python_module(deps: &mut Vec<PathBuf>, module: &str, dir: &Path, project_root: &Path) {
    // Convert dotted module name to path
    let parts: Vec<&str> = module.split('.').collect();

    // Try relative to current directory first
    let mut candidate = dir.to_path_buf();
    for part in &parts {
        candidate = candidate.join(part);
    }

    // Try module.py
    let with_py = candidate.with_extension("py");
    if with_py.is_file() && with_py.starts_with(project_root) {
        deps.push(with_py);
        return;
    }

    // Try module/__init__.py
    let with_init = candidate.join("__init__.py");
    if with_init.is_file() && with_init.starts_with(project_root) {
        deps.push(with_init);
    }
}

/// For JavaScript/TypeScript files, find import dependencies that live in this project.
///
/// Parses `import ... from './foo'` statements and resolves relative imports to actual files.
pub fn find_js_ts_dependencies(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    let mut deps = Vec::new();
    let dir = file_path.parent().unwrap_or(project_root);

    // Detect if this is TypeScript or JavaScript
    let is_ts = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "ts" || e == "tsx")
        .unwrap_or(false);

    let language = if is_ts {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    } else {
        tree_sitter_javascript::LANGUAGE.into()
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return deps;
    }

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return deps,
    };

    // Query for import statements
    let query_str = r#"
        (import_statement
            source: (string) @import_source
        )
    "#;

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(_) => return deps,
    };

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            if let Ok(import_spec) = capture.node.utf8_text(source.as_bytes()) {
                // Remove quotes
                let spec = import_spec.trim_matches(|c| c == '"' || c == '\'');

                // Only process relative imports (starting with ./ or ../)
                if spec.starts_with('.') {
                    if let Some(resolved) = resolve_js_ts_spec(spec, dir, project_root) {
                        deps.push(resolved);
                    }
                }
            }
        }
    }

    deps
}

fn resolve_js_ts_spec(spec: &str, dir: &Path, project_root: &Path) -> Option<PathBuf> {
    let candidate = dir.join(spec);

    // Try with various extensions
    for ext in &["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
        let with_ext = candidate.with_extension(ext);
        if with_ext.is_file() && with_ext.starts_with(project_root) {
            return Some(with_ext);
        }
    }

    // Try as directory with index file
    for ext in &["ts", "tsx", "js", "jsx"] {
        let index = candidate.join(format!("index.{ext}"));
        if index.is_file() && index.starts_with(project_root) {
            return Some(index);
        }
    }

    // Try exact path
    if candidate.is_file() && candidate.starts_with(project_root) {
        return Some(candidate);
    }

    None
}

// ============================================================================
// Template Support (Askama/Jinja2)
// ============================================================================

use regex::Regex;

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
}

/// Find templates directory by walking up from file path
///
/// Searches up to MAX_DEPTH parent directories to avoid performance issues
/// in deeply nested projects.
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
        dependencies.push(TemplateDependency {
            path: template_path.to_string_lossy().to_string(),
            dependency_type: "extends".to_string(),
            name: template_name.to_string(),
        });
    }

    // Find includes
    for cap in include_re.captures_iter(source) {
        let template_name = &cap[1];
        let template_path = templates_dir.join(template_name);
        dependencies.push(TemplateDependency {
            path: template_path.to_string_lossy().to_string(),
            dependency_type: "include".to_string(),
            name: template_name.to_string(),
        });
    }

    Ok(dependencies)
}

/// Recursively merge a template with its parent templates and includes
///
/// Handles {% extends %} and {% include %} directives, merging content appropriately.
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
