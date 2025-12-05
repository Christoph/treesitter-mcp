//! Query Pattern Tool
//!
//! Executes custom tree-sitter queries on source files.
//! Allows users to specify arbitrary S-expression patterns to extract information.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use tree_sitter::{Query, QueryCursor};

#[derive(Debug, serde::Serialize)]
struct QueryResult {
    query: String,
    matches: Vec<QueryMatch>,
}

#[derive(Debug, serde::Serialize)]
struct QueryMatch {
    line: usize,
    column: usize,
    text: String,
    captures: HashMap<String, String>,
}

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
            format!("Failed to read file '{}': {}", file_path, e),
        )
    })?;

    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file '{}': {}", file_path, e),
        )
    })?;
    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse {} code from file '{}': {}",
                language.name(),
                file_path,
                e
            ),
        )
    })?;

    let ts_language = language.tree_sitter_language();
    let query = Query::new(&ts_language, query_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Failed to parse query '{}': {}", query_str, e),
        )
    })?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut results = Vec::new();

    for query_match in matches {
        let mut captures_map = HashMap::new();
        let mut first_capture = None;

        for capture in query_match.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            let node = capture.node;

            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                captures_map.insert(capture_name.to_string(), text.to_string());

                if first_capture.is_none() {
                    first_capture = Some(node);
                }
            }
        }

        if let Some(node) = first_capture {
            let start_pos = node.start_position();
            results.push(QueryMatch {
                line: start_pos.row + 1,
                column: start_pos.column + 1,
                text: node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
                captures: captures_map,
            });
        }
    }

    let result = QueryResult {
        query: query_str.to_string(),
        matches: results,
    };

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to serialize query results for query '{}' on file '{}': {}",
                query_str, file_path, e
            ),
        )
    })?;
    Ok(CallToolResult::success(result_json))
}
