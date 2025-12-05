//! Get Context Tool
//!
//! Returns the enclosing context (function, class, module) at a specific position.
//! Walks up the syntax tree from the given position to collect all enclosing contexts.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::{json, Value};
use std::fs;
use std::io;
use tree_sitter::Node;

/// Context information for a single enclosing scope
#[derive(Debug, serde::Serialize)]
struct Context {
    #[serde(rename = "type")]
    node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<ContextRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

/// Range information for a context
#[derive(Debug, serde::Serialize)]
struct ContextRange {
    start: Position,
    end: Position,
}

/// Position information (line and column)
#[derive(Debug, serde::Serialize)]
struct Position {
    line: u32,
    column: u32,
}

/// Execute the get_context tool
///
/// # Arguments
/// * `arguments` - JSON object with:
///   - `file_path`: String - Path to the source file
///   - `line`: u32 - 1-indexed line number
///   - `column`: Option<u32> - 1-indexed column number (default: 1)
///
/// # Returns
/// Returns a `CallToolResult` with JSON containing:
/// - `file`: File path
/// - `position`: The queried position
/// - `contexts`: Array of enclosing contexts from innermost to outermost
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
            "Missing or invalid 'line' argument",
        )
    })? as u32;

    let column = arguments["column"].as_u64().map(|c| c as u32).unwrap_or(1);

    log::info!("Getting context for {file_path} at line {line}, column {column}");

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

    // Convert 1-indexed line/column to 0-indexed for tree-sitter
    let ts_line = if line > 0 { (line - 1) as usize } else { 0 };
    let ts_column = if column > 0 { (column - 1) as usize } else { 0 };

    // Find the node at the given position
    let node = find_node_at_position(&tree, ts_line, ts_column);

    // Collect enclosing contexts
    let contexts = if let Some(node) = node {
        collect_enclosing_contexts(node, &source, language)
    } else {
        // If no node found at position, return only source_file
        vec![Context {
            node_type: "source_file".to_string(),
            name: Some(
                std::path::Path::new(file_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string(),
            ),
            signature: None,
            range: None,
            code: None,
        }]
    };

    let result = json!({
        "file": file_path,
        "position": {
            "line": line,
            "column": column
        },
        "contexts": contexts
    });

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}

/// Find the deepest node at the given position
fn find_node_at_position(tree: &tree_sitter::Tree, line: usize, column: usize) -> Option<Node<'_>> {
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

    // Return the node if it's not the root
    if node.kind() != "source_file" {
        Some(node)
    } else {
        None
    }
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

/// Collect all enclosing contexts from innermost to outermost
fn collect_enclosing_contexts(
    mut node: Node,
    source: &str,
    language: crate::parser::Language,
) -> Vec<Context> {
    let mut contexts = Vec::new();
    let mut is_innermost = true;

    loop {
        if is_context_node(node.kind(), language) {
            let context = extract_context_info(node, source, is_innermost);
            contexts.push(context);
            is_innermost = false;
        }

        if let Some(parent) = node.parent() {
            node = parent;
        } else {
            break;
        }
    }

    // Add source_file as the outermost context only if no other contexts were found
    if contexts.is_empty() {
        contexts.push(Context {
            node_type: "source_file".to_string(),
            name: None,
            signature: None,
            range: None,
            code: None,
        });
    }

    contexts
}

/// Check if a node type represents a context (function, class, module, etc.)
fn is_context_node(node_type: &str, language: crate::parser::Language) -> bool {
    match language {
        crate::parser::Language::Rust => matches!(
            node_type,
            "function_item"
                | "impl_item"
                | "mod_item"
                | "closure_expression"
                | "struct_item"
                | "enum_item"
                | "trait_item"
        ),
        crate::parser::Language::Python => matches!(
            node_type,
            "function_definition" | "class_definition" | "async_function_definition"
        ),
        crate::parser::Language::JavaScript | crate::parser::Language::TypeScript => matches!(
            node_type,
            "function_declaration"
                | "arrow_function"
                | "class_declaration"
                | "method_definition"
                | "function_expression"
                | "async_function_declaration"
                | "interface_declaration"
        ),
        _ => false,
    }
}

/// Extract context information from a node
fn extract_context_info(node: Node, source: &str, include_code: bool) -> Context {
    let mut node_type = node.kind().to_string();

    // Normalize node types across languages
    node_type = match node_type.as_str() {
        "class_definition" => "class_declaration".to_string(),
        "async_function_definition" => "async_function_declaration".to_string(),
        _ => node_type,
    };

    let start_pos = node.start_position();
    let end_pos = node.end_position();

    let name = extract_name(node, source);
    let signature = if should_have_signature(&node_type) {
        extract_signature(node, source)
    } else {
        None
    };

    let code = if include_code {
        extract_code(node, source)
    } else {
        None
    };

    let range = ContextRange {
        start: Position {
            line: (start_pos.row + 1) as u32,
            column: (start_pos.column + 1) as u32,
        },
        end: Position {
            line: (end_pos.row + 1) as u32,
            column: (end_pos.column + 1) as u32,
        },
    };

    Context {
        node_type,
        name,
        signature,
        range: Some(range),
        code,
    }
}

/// Extract the name of a node (function name, class name, etc.)
fn extract_name(node: Node, source: &str) -> Option<String> {
    // Try to find a child node with type "identifier" or "type_identifier"
    for child in node.children(&mut node.walk()) {
        if matches!(
            child.kind(),
            "identifier" | "type_identifier" | "property_identifier"
        ) {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }

    // For some node types, try specific patterns
    match node.kind() {
        "impl_item" => {
            // impl blocks have the type name as a child
            for child in node.children(&mut node.walk()) {
                if matches!(child.kind(), "type_identifier" | "generic_type") {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return Some(text.to_string());
                    }
                }
            }
        }
        "interface_declaration" => {
            // TypeScript interfaces
            for child in node.children(&mut node.walk()) {
                if child.kind() == "type_identifier" {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return Some(text.to_string());
                    }
                }
            }
        }
        "method_definition" => {
            // JavaScript/TypeScript methods have property_identifier
            for child in node.children(&mut node.walk()) {
                if child.kind() == "property_identifier" {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        return Some(text.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    None
}

/// Check if a node type should have a signature
fn should_have_signature(node_type: &str) -> bool {
    matches!(
        node_type,
        "function_item"
            | "function_declaration"
            | "arrow_function"
            | "function_expression"
            | "async_function_declaration"
            | "method_definition"
            | "function_definition"
            | "async_function_definition"
            | "closure_expression"
    )
}

/// Extract the signature of a function
fn extract_signature(node: Node, source: &str) -> Option<String> {
    // For most functions, the signature is from the start to the opening brace
    let start = node.start_byte();
    let mut end = node.start_byte();

    // Find the opening brace or colon (for Python)
    let source_bytes = source.as_bytes();
    let mut paren_depth = 0;
    let mut bracket_depth = 0;
    let mut found_brace = false;

    for (i, &byte) in source_bytes[start..].iter().enumerate() {
        let byte_pos = start + i;

        match byte {
            b'(' => paren_depth += 1,
            b')' => paren_depth -= 1,
            b'[' => bracket_depth += 1,
            b']' => bracket_depth -= 1,
            b'{' | b':' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    end = byte_pos;
                    found_brace = true;
                    break;
                }
            }
            _ => {}
        }
    }

    if found_brace {
        if let Ok(sig) = std::str::from_utf8(&source_bytes[start..end]) {
            return Some(sig.trim().to_string());
        }
    }

    // Fallback: return the first line of the node
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
        let first_line = text.lines().next().unwrap_or("");
        if !first_line.is_empty() {
            return Some(first_line.to_string());
        }
    }

    None
}

/// Extract the full code of a node
fn extract_code(node: Node, source: &str) -> Option<String> {
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}
