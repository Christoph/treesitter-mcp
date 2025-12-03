//! Code Map Tool
//!
//! Generates a high-level overview of a codebase with token budget awareness.
//! Walks directory structure, extracts file shapes, and aggregates results.

use crate::mcp::types::{CallToolResult, ToolDefinition};
use crate::parser::detect_language;
use eyre::{Result, WrapErr};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Approximate tokens per character (rough estimation)
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, serde::Serialize)]
struct CodeMap {
    files: Vec<FileEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncated: Option<bool>,
}

#[derive(Debug, serde::Serialize)]
struct FileEntry {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    functions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    structs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    classes: Option<Vec<String>>,
}

pub fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "code_map".to_string(),
        description: "Use this tool to build a mental model of the project's architecture. The intent is to generate a token-efficient, high-level summary of an entire directory tree, helping you identify key files and modules. Use this when you need to explore a new codebase or locate where specific functionality resides across multiple files.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to file or directory"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Maximum tokens for output (approximate)",
                    "default": 2000
                }
            },
            "required": ["path"]
        }),
    }
}

pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    let path_str = arguments["path"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing 'path' argument"))?;

    let max_tokens = arguments["max_tokens"].as_i64().unwrap_or(2000) as usize;

    log::info!("Generating code map for: {path_str} (max_tokens: {max_tokens})");

    let path = Path::new(path_str);

    if !path.exists() {
        return Err(eyre::eyre!("Path does not exist: {}", path_str));
    }

    let mut files = Vec::new();
    let mut current_tokens = 0;
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    let mut truncated = false;

    if path.is_file() {
        // Single file
        if let Ok(entry) = process_file(path) {
            files.push(entry);
        }
    } else if path.is_dir() {
        // Directory - walk and collect
        collect_files(
            path,
            &mut files,
            &mut current_tokens,
            max_chars,
            &mut truncated,
        )?;
    }

    let code_map = CodeMap {
        files,
        truncated: if truncated { Some(true) } else { None },
    };

    let map_json = serde_json::to_string_pretty(&code_map)?;
    Ok(CallToolResult::success(map_json))
}

/// Recursively collect files from directory
fn collect_files(
    dir: &Path,
    files: &mut Vec<FileEntry>,
    current_tokens: &mut usize,
    max_chars: usize,
    truncated: &mut bool,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .wrap_err_with(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden files and common ignore patterns
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
                continue;
            }
        }

        if path.is_file() {
            // Check if we can detect language (skip non-source files)
            if detect_language(&path).is_ok() {
                if let Ok(entry) = process_file(&path) {
                    let entry_json = serde_json::to_string(&entry)?;
                    let entry_size = entry_json.len();

                    if *current_tokens + entry_size > max_chars {
                        *truncated = true;
                        break;
                    }

                    *current_tokens += entry_size;
                    files.push(entry);
                }
            }
        } else if path.is_dir() {
            collect_files(&path, files, current_tokens, max_chars, truncated)?;
            if *truncated {
                break;
            }
        }
    }

    Ok(())
}

/// Process a single file and extract its shape
fn process_file(path: &Path) -> Result<FileEntry> {
    let source = fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read file: {}", path.display()))?;

    let language = detect_language(path)?;
    let tree = crate::parser::parse_code(&source, language)?;

    // Use the file_shape extraction logic
    let shape = crate::analysis::file_shape::extract_shape(&tree, &source, language)?;

    let path_str = path.to_string_lossy().to_string();

    let functions = if !shape.functions.is_empty() {
        Some(shape.functions.iter().map(|f| f.name.clone()).collect())
    } else {
        None
    };

    let structs = if !shape.structs.is_empty() {
        Some(shape.structs.iter().map(|s| s.name.clone()).collect())
    } else {
        None
    };

    let classes = if !shape.classes.is_empty() {
        Some(shape.classes.iter().map(|c| c.name.clone()).collect())
    } else {
        None
    };

    Ok(FileEntry {
        path: path_str,
        functions,
        structs,
        classes,
    })
}
