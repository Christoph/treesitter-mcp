//! File Shape Tool
//!
//! Extracts the high-level structure of a source file (functions, classes, imports)
//! without the implementation details.

use eyre::{Result, WrapErr};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{Query, QueryCursor, Tree};
use crate::mcp::types::{CallToolResult, ToolDefinition};
use crate::parser::{detect_language, parse_code, Language};
use serde_json::{json, Value};

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

pub fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "file_shape".to_string(),
        description: "Use this tool to 'skeletonize' code files. The intent is to quickly understand the interface (functions, classes, structs) and dependencies (imports) of a file without reading the full implementation. This is primarily used for generating file summaries, mapping dependency graphs, and understanding the high-level organization of code.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the source file"
                },
                "include_deps": {
                    "type": "boolean",
                    "description": "Include project dependencies as nested file shapes",
                    "default": false
                }
            },
            "required": ["file_path"]
        }),
    }
}

pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    let file_path_str = arguments["file_path"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing 'file_path' argument"))?;

    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);

    log::info!(
        "Extracting shape of file: {file_path_str} (include_deps: {include_deps})"
    );

    let path = Path::new(file_path_str);

    // Determine project root (directory containing Cargo.toml if present)
    let project_root = find_project_root(path).unwrap_or_else(|| {
        path.parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    });

    let mut visited = HashSet::new();
    let shape = build_shape_tree(path, &project_root, include_deps, &mut visited)?;
    let shape_json = serde_json::to_string_pretty(&shape)?;

    Ok(CallToolResult::success(shape_json))
}

pub fn extract_shape(tree: &Tree, source: &str, language: Language) -> Result<FileShape> {
    match language {
        Language::Rust => extract_rust_shape(tree, source),
        Language::Python => extract_python_shape(tree, source),
        Language::JavaScript | Language::TypeScript => extract_js_shape(tree, source),
        _ => Ok(FileShape {
            path: None,
            functions: vec![],
            structs: vec![],
            classes: vec![],
            imports: vec![],
            dependencies: vec![],
        }),
    }
}

fn extract_rust_shape(tree: &Tree, source: &str) -> Result<FileShape> {
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
    )?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    functions.push(FunctionInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "struct.name" => {
                    structs.push(StructInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "import" => {
                    imports.push(node.utf8_text(source.as_bytes())?.to_string());
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

fn extract_python_shape(tree: &Tree, source: &str) -> Result<FileShape> {
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
    )?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    functions.push(FunctionInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "class.name" => {
                    classes.push(ClassInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "import" => {
                    imports.push(node.utf8_text(source.as_bytes())?.to_string());
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

fn extract_js_shape(tree: &Tree, source: &str) -> Result<FileShape> {
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
    )?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    for match_ in matches {
        for capture in match_.captures {
            let node = capture.node;
            let name = capture.index;

            match query.capture_names()[name as usize] {
                "func.name" => {
                    functions.push(FunctionInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "class.name" => {
                    classes.push(ClassInfo {
                        name: node.utf8_text(source.as_bytes())?.to_string(),
                        line: node.start_position().row + 1,
                    });
                }
                "import" => {
                    imports.push(node.utf8_text(source.as_bytes())?.to_string());
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
) -> Result<FileShape> {
    // Avoid infinite recursion in case of cyclic module structures
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        // Already processed – just return the flat shape
        let source = fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read file: {}", path.display()))?;
        let language = detect_language(path)?;
        let tree = parse_code(&source, language)?;
        let mut shape = extract_shape(&tree, &source, language)?;
        shape.path = Some(path.to_string_lossy().to_string());
        return Ok(shape);
    }
    visited.insert(canonical);

    let source = fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read file: {}", path.display()))?;

    let language = detect_language(path)?;
    let tree = parse_code(&source, language)?;

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
/// This currently looks for `mod foo;` or `pub mod foo;` declarations and resolves
/// them to `foo.rs` or `foo/mod.rs` under the same directory, constrained to
/// `project_root` so that only project files are included.
fn find_rust_dependencies(source: &str, file_path: &Path, project_root: &Path) -> Vec<PathBuf> {
    let mut deps = Vec::new();

    let dir = file_path
        .parent()
        .unwrap_or(project_root);

    for line in source.lines() {
        let trimmed = line.trim_start();

        if !(trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ")) {
            continue;
        }

        // Normalize to start after `mod `
        let after_pub = if let Some(rest) = trimmed.strip_prefix("pub ") {
            rest
        } else {
            trimmed
        };

        let after_mod = if let Some(rest) = after_pub.strip_prefix("mod ") {
            rest
        } else {
            continue;
        };

        let name: String = after_mod
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if name.is_empty() {
            continue;
        }

        let candidate_files = [
            dir.join(format!("{name}.rs")),
            dir.join(name).join("mod.rs"),
        ];

        for candidate in candidate_files {
            if candidate.is_file() && candidate.starts_with(project_root) {
                deps.push(candidate);
                break;
            }
        }
    }

    deps
}

/// For Python files, find simple module dependencies that live in this project.
///
/// This currently looks for `import foo` style imports and resolves them to
/// `foo.py` or `foo/__init__.py` either next to the file or under the
/// project root, constrained to `project_root`.
fn find_python_dependencies(source: &str, file_path: &Path, project_root: &Path) -> Vec<PathBuf> {
    let mut deps = Vec::new();

    let dir = file_path
        .parent()
        .unwrap_or(project_root);

    for line in source.lines() {
        let trimmed = line.trim_start();

        if let Some(rest) = trimmed.strip_prefix("import ") {
            for part in rest.split(',') {
                let name = part.split_whitespace().next().unwrap_or("");
                if name.is_empty() {
                    continue;
                }

                push_python_module(&mut deps, name, dir, project_root);
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            let module = rest.split_whitespace().next().unwrap_or("");
            if module.is_empty() {
                continue;
            }

            // Ignore relative imports like `from . import foo` for now
            if module.starts_with('.') {
                continue;
            }

            push_python_module(&mut deps, module, dir, project_root);
        }
    }

    deps
}

fn push_python_module(
    deps: &mut Vec<PathBuf>,
    module: &str,
    dir: &Path,
    project_root: &Path,
) {
    let base = module.split('.').next().unwrap_or(module);

    let candidate_files = [
        dir.join(format!("{base}.py")),
        project_root.join(format!("{base}.py")),
        dir.join(base).join("__init__.py"),
        project_root.join(base).join("__init__.py"),
    ];

    for candidate in candidate_files {
        if candidate.is_file() && candidate.starts_with(project_root) {
            deps.push(candidate);
            break;
        }
    }
}

/// For JavaScript/TypeScript files, find relative import dependencies that live
/// in this project.
///
/// This looks for ESM-style `import` statements with a string literal module
/// specifier and resolves relative paths like `./utils.js` against the current
/// file directory, constrained to `project_root`.
fn find_js_ts_dependencies(source: &str, file_path: &Path, project_root: &Path) -> Vec<PathBuf> {
    let mut deps = Vec::new();

    let dir = file_path
        .parent()
        .unwrap_or(project_root);

    for line in source.lines() {
        let trimmed = line.trim_start();

        // Handle `import ... from "module"` and `export ... from "module"`
        if let Some(idx) = trimmed.find(" from ") {
            let after = &trimmed[idx + " from ".len()..];
            if let Some(spec) = extract_string_literal(after) {
                if let Some(candidate) = resolve_js_ts_spec(&spec, dir, project_root) {
                    deps.push(candidate);
                }
            }
            continue;
        }

        // Handle bare side-effect imports: `import "module";`
        if let Some(after) = trimmed.strip_prefix("import ") {
            if let Some(spec) = extract_string_literal(after) {
                if let Some(candidate) = resolve_js_ts_spec(&spec, dir, project_root) {
                    deps.push(candidate);
                }
            }
        }
    }

    deps
}

fn extract_string_literal(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut i = 0;

    // Find first quote
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let start = i;
            while i < bytes.len() {
                let c2 = bytes[i] as char;
                if c2 == quote {
                    let end = i;
                    return Some(source[start..end].to_string());
                }
                i += 1;
            }
            break;
        }
        i += 1;
    }

    None
}

fn resolve_js_ts_spec(spec: &str, dir: &Path, project_root: &Path) -> Option<PathBuf> {
    // Only consider relative imports; skip bare module specifiers so that we
    // don't accidentally include external dependencies.
    if !(spec.starts_with("./") || spec.starts_with("../")) {
        return None;
    }

    let candidate = dir.join(spec);

    // If the specifier has no extension, try common JS/TS extensions
    if candidate.extension().is_none() {
        for ext in &["js", "jsx", "ts", "tsx"] {
            let with_ext = candidate.with_extension(ext);
            if with_ext.is_file() && with_ext.starts_with(project_root) {
                return Some(with_ext);
            }
        }
    }

    if candidate.is_file() && candidate.starts_with(project_root) {
        return Some(candidate);
    }

    None
}
