//! Query Pattern Tool
//!
//! Executes custom tree-sitter queries on source files.
//! Allows users to specify arbitrary S-expression patterns to extract information.

use crate::mcp::types::{CallToolResult, ToolDefinition};
use crate::parser::{detect_language, parse_code};
use eyre::{Result, WrapErr};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
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

pub fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "query_pattern".to_string(),
        description: "Use this tool to perform surgical, structural search operations on code. The intent is to extract specific syntax patterns (e.g., 'all public functions returning Result') that regular expression searches cannot handle. Use this for advanced static analysis or custom data extraction tasks.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the source file"
                },
                "query": {
                    "type": "string",
                    "description": "Tree-sitter query pattern in S-expression format"
                }
            },
            "required": ["file_path", "query"]
        }),
    }
}

pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    let file_path = arguments["file_path"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing 'file_path' argument"))?;

    let query_str = arguments["query"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing 'query' argument"))?;

    log::info!("Executing query on file: {file_path}");

    let source = fs::read_to_string(file_path)
        .wrap_err_with(|| format!("Failed to read file: {file_path}"))?;

    let language = detect_language(file_path)?;
    let tree = parse_code(&source, language)?;

    let ts_language = language.tree_sitter_language();
    let query = Query::new(&ts_language, query_str).wrap_err("Failed to parse query")?;

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

    let result_json = serde_json::to_string_pretty(&result)?;
    Ok(CallToolResult::success(result_json))
}
