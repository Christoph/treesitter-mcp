//! Parse File Tool
//!
//! This tool parses a source file using tree-sitter and returns the
//! file shape - structured JSON with functions, classes, imports, and their signatures.

use crate::analysis::shape::extract_enhanced_shape;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::fs;
use std::io;

/// Execute the parse_file tool
///
/// # Arguments
/// * `arguments` - JSON object with `file_path` field
///
/// # Returns
/// Returns a `CallToolResult` with structured JSON containing:
/// - `path`: File path
/// - `language`: Detected language
/// - `functions`: Array of function definitions with signatures and code
/// - `structs`: Array of struct definitions (Rust)
/// - `classes`: Array of class definitions (Python, JavaScript, TypeScript)
/// - `imports`: Array of import statements
///
/// # Errors
/// Returns an error if:
/// - The `file_path` argument is missing or invalid
/// - The file cannot be read
/// - The file extension is not supported
/// - Parsing fails completely (very rare)
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    log::info!("Parsing file: {file_path}");

    let source = fs::read_to_string(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file {file_path}: {e}"),
        )
    })?;

    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file {file_path}: {e}"),
        )
    })?;

    log::debug!("Detected language: {}", language.name());

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let shape = extract_enhanced_shape(&tree, &source, language, Some(file_path))?;

    log::debug!(
        "Extracted file shape with {} functions",
        shape.functions.len()
    );

    let shape_json = serde_json::to_string(&shape).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize shape to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(shape_json))
}
