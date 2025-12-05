//! Get Node At Position Tool
//!
//! This tool gets the AST node at a specific position with ancestor chain.
//! Returns the smallest node at the position plus ancestor nodes up to a specified level.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::{json, Value};
use std::fs;
use std::io;
use tree_sitter::{Node, Tree};

/// Execute the get_node_at_position tool
///
/// # Arguments
/// * `arguments` - JSON object with:
///   - `file_path`: String - Path to the source file
///   - `line`: u32 - 1-indexed line number
///   - `column`: u32 - 1-indexed column number
///   - `ancestor_levels`: Option<u32> - Number of ancestor levels to return (default: 3)
///
/// # Returns
/// Returns a `CallToolResult` with JSON containing:
/// - `file`: File path
/// - `position`: Object with `line` and `column`
/// - `node`: Object with `type`, `text` (if small), and `range`
/// - `ancestors`: Array of ancestor nodes with same structure
///
/// # Errors
/// Returns an error if:
/// - Required arguments are missing or invalid
/// - The file cannot be read
/// - The file extension is not supported
/// - The position is beyond file bounds
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let line = arguments["line"].as_u64().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'line' argument (must be u32)",
        )
    })? as u32;

    let column = arguments["column"].as_u64().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'column' argument (must be u32)",
        )
    })? as u32;

    let ancestor_levels = arguments["ancestor_levels"]
        .as_u64()
        .map(|v| v as u32)
        .unwrap_or(3);

    log::info!(
        "Getting node at position: {}:{}:{}",
        file_path,
        line,
        column
    );

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

    // Line is 0-indexed, column is 1-indexed
    let ts_line = line as usize;
    let ts_column = if column > 0 { (column - 1) as usize } else { 0 };

    // Find the smallest node at the position
    let node = find_smallest_node_at_position(&tree, ts_line, ts_column).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("No node found at position {}:{}", line, column),
        )
    })?;

    // Extract node information
    let node_info = extract_node_info(node, &source)?;

    // Collect ancestors
    let ancestors = if ancestor_levels > 0 {
        collect_ancestors(node, ancestor_levels, &source)?
    } else {
        vec![]
    };

    // Build response JSON
    let response = json!({
        "file": file_path,
        "position": {
            "line": line,
            "column": column
        },
        "node": node_info,
        "ancestors": ancestors
    });

    let response_json = serde_json::to_string(&response).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize response to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(response_json))
}

/// Find the smallest node at the given position
fn find_smallest_node_at_position(tree: &Tree, line: usize, column: usize) -> Option<Node<'_>> {
    let mut node = tree.root_node();

    loop {
        let mut found_child = false;

        for child in node.children(&mut node.walk()) {
            let start = child.start_position();
            let end = child.end_position();

            // Check if position is within this child's range
            if is_position_in_range(line, column, start, end) {
                node = child;
                found_child = true;
                break;
            }
        }

        if !found_child {
            break;
        }
    }

    Some(node)
}

/// Check if a position is within a range
fn is_position_in_range(
    line: usize,
    column: usize,
    start: tree_sitter::Point,
    end: tree_sitter::Point,
) -> bool {
    if line < start.row || line > end.row {
        return false;
    }

    if line == start.row && column < start.column {
        return false;
    }

    if line == end.row && column >= end.column {
        return false;
    }

    true
}

/// Collect ancestor nodes up to a specified level
fn collect_ancestors(node: Node, levels: u32, source: &str) -> Result<Vec<Value>, io::Error> {
    let mut ancestors = Vec::new();
    let mut current = node;
    let mut count = 0;

    while let Some(parent) = current.parent() {
        if count >= levels {
            break;
        }

        let ancestor_info = extract_node_info(parent, source)?;
        ancestors.push(ancestor_info);

        current = parent;
        count += 1;
    }

    Ok(ancestors)
}

/// Extract node information as JSON
fn extract_node_info(node: Node, source: &str) -> Result<Value, io::Error> {
    let node_type = node.kind();
    let start = node.start_position();
    let end = node.end_position();

    // Try to extract text if it's small enough
    let text = if node.child_count() == 0 {
        // Leaf node - extract text
        node.utf8_text(source.as_bytes())
            .ok()
            .map(|s| s.to_string())
    } else {
        // Non-leaf node - extract text if small enough
        let text_result = node.utf8_text(source.as_bytes());
        match text_result {
            Ok(text) if text.len() < 100 => Some(text.to_string()),
            _ => None,
        }
    };

    // Try to extract name for named constructs
    let name = extract_node_name(node, source);

    let mut info = json!({
        "type": node_type,
        "range": {
            "start": {
                "line": start.row + 1,
                "column": start.column
            },
            "end": {
                "line": end.row + 1,
                "column": end.column
            }
        }
    });

    // Add text if available
    if let Some(text) = text {
        info["text"] = Value::String(text);
    }

    // Add name if available
    if let Some(name) = name {
        info["name"] = Value::String(name);
    }

    Ok(info)
}

/// Extract name from a node if applicable
fn extract_node_name(node: Node, source: &str) -> Option<String> {
    // For named nodes, try to find the name child
    for child in node.children(&mut node.walk()) {
        if child.kind() == "identifier" || child.kind() == "type_identifier" {
            if let Ok(name) = child.utf8_text(source.as_bytes()) {
                return Some(name.to_string());
            }
        }
    }

    // For function_item, look for the name field
    if node.kind() == "function_item" {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                if let Ok(name) = child.utf8_text(source.as_bytes()) {
                    return Some(name.to_string());
                }
            }
        }
    }

    None
}
