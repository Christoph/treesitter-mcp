//! Find Usages Tool
//!
//! Searches for all usages of a symbol (function, struct, class) across files.
//! Uses tree-sitter to parse and search for identifier nodes.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::Path;
use tree_sitter::Tree;

#[derive(Debug, serde::Serialize)]
struct FindUsagesResult {
    symbol: String,
    usages: Vec<Usage>,
}

#[derive(Debug, serde::Serialize)]
struct Usage {
    file: String,
    line: usize,
    column: usize,
    context: String,
}

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let symbol = arguments["symbol"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'symbol' argument",
        )
    })?;

    let path_str = arguments["path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'path' argument",
        )
    })?;

    log::info!("Finding usages of '{symbol}' in: {path_str}");

    let path = Path::new(path_str);

    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {}", path_str),
        ));
    }

    let mut usages = Vec::new();

    if path.is_file() {
        search_file(path, symbol, &mut usages)?;
    } else if path.is_dir() {
        search_directory(path, symbol, &mut usages)?;
    }

    let result = FindUsagesResult {
        symbol: symbol.to_string(),
        usages,
    };

    let result_json = serde_json::to_string_pretty(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;
    Ok(CallToolResult::success(result_json))
}

/// Recursively search directory for symbol usages
fn search_directory(dir: &Path, symbol: &str, usages: &mut Vec<Usage>) -> Result<(), io::Error> {
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
            // Only search files with detectable language
            if detect_language(&path).is_ok() {
                let _ = search_file(&path, symbol, usages);
            }
        } else if path.is_dir() {
            search_directory(&path, symbol, usages)?;
        }
    }

    Ok(())
}

/// Search for symbol usages in a single file
fn search_file(path: &Path, symbol: &str, usages: &mut Vec<Usage>) -> Result<(), io::Error> {
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

    find_identifiers(&tree, &source, symbol, path, usages);

    Ok(())
}

/// Find all identifier nodes matching the symbol name
fn find_identifiers(tree: &Tree, source: &str, symbol: &str, path: &Path, usages: &mut Vec<Usage>) {
    let root = tree.root_node();
    let mut cursor = root.walk();

    visit_node(&mut cursor, source, symbol, path, usages);
}

/// Recursively visit nodes to find matching identifiers
fn visit_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    symbol: &str,
    path: &Path,
    usages: &mut Vec<Usage>,
) {
    let node = cursor.node();

    // Check if this node is an identifier that matches our symbol
    if node.kind() == "identifier" || node.kind().ends_with("_identifier") {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if text == symbol {
                let start_pos = node.start_position();
                let line_text = get_line(source, start_pos.row);

                usages.push(Usage {
                    file: path.to_string_lossy().to_string(),
                    line: start_pos.row + 1,
                    column: start_pos.column + 1,
                    context: line_text.trim().to_string(),
                });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            visit_node(cursor, source, symbol, path, usages);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Get a specific line from source text
fn get_line(source: &str, line_num: usize) -> String {
    source.lines().nth(line_num).unwrap_or("").to_string()
}
