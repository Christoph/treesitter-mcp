//! Code Map Tool
//!
//! Generates a high-level overview of a codebase with token budget awareness.
//! Walks directory structure, extracts file shapes, and aggregates results.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::detect_language;
use serde_json::Value;
use std::fs;
use std::io;
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

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let path_str = arguments["path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'path' argument",
        )
    })?;

    let max_tokens = arguments["max_tokens"].as_i64().unwrap_or(2000) as usize;

    log::info!("Generating code map for: {path_str} (max_tokens: {max_tokens})");

    let path = Path::new(path_str);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {}", path_str),
        ));
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

    let map_json = serde_json::to_string_pretty(&code_map).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize code map to JSON: {e}"),
        )
    })?;
    Ok(CallToolResult::success(map_json))
}

/// Recursively collect files from directory
fn collect_files(
    dir: &Path,
    files: &mut Vec<FileEntry>,
    current_tokens: &mut usize,
    max_chars: usize,
    truncated: &mut bool,
) -> Result<(), io::Error> {
    let entries = fs::read_dir(dir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read directory {}: {e}", dir.display()),
        )
    })?;

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
                    let entry_json = serde_json::to_string(&entry).map_err(|e| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Failed to serialize entry to JSON: {e}"),
                        )
                    })?;
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
fn process_file(path: &Path) -> Result<FileEntry, io::Error> {
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
    let tree = crate::parser::parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

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
