//! File Shape Tool
//!
//! Extracts the high-level structure of a source file (functions, classes, imports)
//! without the implementation details.

use eyre::{Result, WrapErr};
use std::fs;
use tree_sitter::{Tree, Query, QueryCursor};
use crate::mcp::types::{CallToolResult, ToolDefinition};
use crate::parser::{detect_language, parse_code, Language};
use serde_json::{json, Value};

#[derive(Debug, serde::Serialize)]
pub struct FileShape {
    pub functions: Vec<FunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<StructInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<ClassInfo>,
    pub imports: Vec<String>,
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
        description: "Extract the structure of a file (functions, classes, imports) without implementation details".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the source file"
                },
                "include_deps": {
                    "type": "boolean",
                    "description": "Include dependencies (not yet implemented)",
                    "default": false
                }
            },
            "required": ["file_path"]
        }),
    }
}

pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    let file_path = arguments["file_path"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing 'file_path' argument"))?;

    log::info!("Extracting shape of file: {}", file_path);

    let source = fs::read_to_string(file_path)
        .wrap_err_with(|| format!("Failed to read file: {}", file_path))?;

    let language = detect_language(file_path)?;
    let tree = parse_code(&source, language)?;

    let shape = extract_shape(&tree, &source, language)?;
    let shape_json = serde_json::to_string_pretty(&shape)?;

    Ok(CallToolResult::success(shape_json))
}

pub fn extract_shape(tree: &Tree, source: &str, language: Language) -> Result<FileShape> {
    match language {
        Language::Rust => extract_rust_shape(tree, source),
        Language::Python => extract_python_shape(tree, source),
        Language::JavaScript | Language::TypeScript => extract_js_shape(tree, source),
        _ => Ok(FileShape {
            functions: vec![],
            structs: vec![],
            classes: vec![],
            imports: vec![],
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
        functions,
        structs,
        classes: vec![],
        imports,
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
        functions,
        structs: vec![],
        classes,
        imports,
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
        functions,
        structs: vec![],
        classes,
        imports,
    })
}
