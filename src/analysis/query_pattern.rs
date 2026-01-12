//! Query Pattern Tool
//!
//! Executes custom tree-sitter queries on source files.
//!
//! Breaking schema change (v1):
//! ```json
//! {
//!   "q": "(function_item name: (identifier) @name)",
//!   "h": "file|line|col|text",
//!   "m": "src/calculator.rs|10|5|add\n..."
//! }
//! ```

use crate::analysis::path_utils;
use crate::common::compact::CompactOutput;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::json;
use serde_json::Value;
use std::fs;
use std::io;
use tree_sitter::{Query, QueryCursor};

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let query_str = arguments["query"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'query' argument",
        )
    })?;

    log::info!("Executing query on file: {file_path}");

    let source = fs::read_to_string(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file '{file_path}': {e}"),
        )
    })?;

    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file '{file_path}': {e}"),
        )
    })?;

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse {} code from file '{file_path}': {e}",
                language.name()
            ),
        )
    })?;

    let ts_language = language.tree_sitter_language();
    let query = Query::new(&ts_language, query_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to parse query '{query_str}': {e}"),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let rel_path = path_utils::to_relative_path(file_path);
    let header = "file|line|col|text";
    let mut out = CompactOutput::new(header);

    for query_match in matches {
        let mut first_capture = None;

        for capture in query_match.captures {
            if first_capture.is_none() {
                first_capture = Some(capture.node);
            }
        }

        if let Some(node) = first_capture {
            let start_pos = node.start_position();
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            let line = (start_pos.row + 1).to_string();
            let col = (start_pos.column + 1).to_string();

            out.add_row(&[&rel_path, &line, &col, text]);
        }
    }

    let result = json!({
        "q": query_str,
        "h": header,
        "m": out.rows_string(),
    });

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to serialize query results for query '{query_str}' on file '{file_path}': {e}"
            ),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}
