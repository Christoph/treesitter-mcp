//! Parse File Tool
//!
//! This tool parses a source file using tree-sitter and returns the
//! Abstract Syntax Tree (AST) as an S-expression.

use crate::mcp::types::{CallToolResult, ToolDefinition};
use crate::parser::{detect_language, parse_code};
use eyre::{Result, WrapErr};
use serde_json::{json, Value};
use std::fs;

/// Create the tool definition for parse_file
///
/// This MCP tool allows clients to parse source code files and receive
/// the tree-sitter AST as an S-expression string.
pub fn tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: "parse_file".to_string(),
        description: "Parse a source file using tree-sitter and return the AST as an S-expression"
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the source file to parse"
                }
            },
            "required": ["file_path"]
        }),
    }
}

/// Execute the parse_file tool
///
/// # Arguments
/// * `arguments` - JSON object with `file_path` field
///
/// # Returns
/// Returns a `CallToolResult` with the S-expression AST as text content.
/// Even files with syntax errors produce a tree (with ERROR nodes).
///
/// # Errors
/// Returns an error if:
/// - The `file_path` argument is missing or invalid
/// - The file cannot be read
/// - The file extension is not supported
/// - Parsing fails completely (very rare)
pub fn execute(arguments: &Value) -> Result<CallToolResult> {
    let file_path = arguments["file_path"]
        .as_str()
        .ok_or_else(|| eyre::eyre!("Missing or invalid 'file_path' argument"))?;

    log::info!("Parsing file: {file_path}");

    let source = fs::read_to_string(file_path)
        .wrap_err_with(|| format!("Failed to read file: {file_path}"))?;

    let language = detect_language(file_path)
        .wrap_err_with(|| format!("Cannot detect language for file: {file_path}"))?;

    log::debug!("Detected language: {}", language.name());

    let tree = parse_code(&source, language)
        .wrap_err_with(|| format!("Failed to parse {} code", language.name()))?;

    let sexp = tree.root_node().to_sexp();

    log::debug!("Generated S-expression ({} bytes)", sexp.len());

    Ok(CallToolResult::success(sexp))
}
