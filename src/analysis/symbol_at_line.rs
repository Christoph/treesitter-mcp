//! Symbol At Line Tool
//!
//! Find what symbol (function/class) is at a specific line with signature and scope chain.
//! Merges functionality from get_context and get_node_at_position.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};
use serde_json::json;
use serde_json::Value;
use std::fs;
use std::io;
use tree_sitter::Node;

#[derive(Debug)]
struct ScopeInfo {
    name: String,
    signature: String,
    kind: String,
}

/// Execute the symbol_at_line tool
///
/// # Arguments
/// * `arguments` - JSON object with:
///   - `file_path`: String - Path to the source file
///   - `line`: u32 - 1-indexed line number
///   - `column`: Option<u32> - 1-indexed column number (default: 1)
///
/// # Returns
/// Returns a `CallToolResult` with JSON containing symbol info and scope chain
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

    log::info!("Getting symbol at {file_path}:{line}:{column}");

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

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    // Convert 1-indexed line/column to 0-indexed for tree-sitter
    let ts_line = if line > 0 { (line - 1) as usize } else { 0 };
    let ts_column = if column > 0 { (column - 1) as usize } else { 0 };

    // Find node at position
    let node = find_node_at_position(&tree, ts_line, ts_column).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("No node found at line {line}, column {column}"),
        )
    })?;

    // Build scope chain - this will walk up from the node to find context nodes
    let scope_chain = collect_scope_chain(node, &source, language);

    if scope_chain.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No symbol found at line {line}, column {column}. The position may be in a comment or whitespace."),
        ));
    }

    // Compact output
    let innermost = &scope_chain[0];

    let scope = scope_chain
        .iter()
        .rev()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join("::");

    let output = json!({
        "sym": innermost.name,
        "kind": abbreviate_kind(&innermost.kind),
        "sig": innermost.signature,
        "l": line,
        "scope": scope,
    });

    let output_json = serde_json::to_string(&output).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize output to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(output_json))
}

/// Find the deepest node at the given position
fn find_node_at_position(tree: &tree_sitter::Tree, line: usize, column: usize) -> Option<Node<'_>> {
    let mut node = tree.root_node();

    loop {
        let mut found_child = false;

        for child in node.children(&mut node.walk()) {
            if is_position_in_range(line, column, child.start_position(), child.end_position()) {
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

    if line == end.row && column > end.column {
        return false;
    }

    true
}

/// Collect scope chain from innermost to outermost
fn collect_scope_chain(mut node: Node, source: &str, language: Language) -> Vec<ScopeInfo> {
    let mut scopes = Vec::new();

    loop {
        if is_context_node(node.kind(), language) {
            if let Some(scope) = extract_scope_info(node, source) {
                scopes.push(scope);
            }
        }

        match node.parent() {
            Some(parent) => node = parent,
            None => break,
        }
    }

    scopes
}

/// Check if a node type represents a context boundary
fn is_context_node(node_type: &str, language: Language) -> bool {
    match language {
        Language::Rust => matches!(
            node_type,
            "function_item" | "impl_item" | "trait_item" | "struct_item" | "enum_item" | "mod_item"
        ),
        Language::Python => matches!(
            node_type,
            "function_definition" | "class_definition" | "module"
        ),
        Language::JavaScript | Language::TypeScript => matches!(
            node_type,
            "function_declaration"
                | "method_definition"
                | "class_declaration"
                | "arrow_function"
                | "function_expression"
        ),
        Language::CSharp => matches!(
            node_type,
            "method_declaration"
                | "constructor_declaration"
                | "property_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "struct_declaration"
                | "namespace_declaration"
        ),
        Language::Java => matches!(
            node_type,
            "method_declaration"
                | "constructor_declaration"
                | "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
        ),
        _ => false,
    }
}

/// Extract scope information from a node
fn extract_scope_info(node: Node, source: &str) -> Option<ScopeInfo> {
    let kind = match node.kind() {
        "function_item" | "function_definition" | "function_declaration" => "function",
        "method_definition" | "method_declaration" => "method",
        "class_definition" | "class_declaration" => "class",
        "impl_item" => "impl",
        "trait_item" => "trait",
        "struct_item" | "struct_declaration" => "struct",
        "enum_item" | "enum_declaration" => "enum",
        "mod_item" | "module" => "module",
        "arrow_function" | "function_expression" => "function",
        "constructor_declaration" => "constructor",
        "property_declaration" => "property",
        "interface_declaration" => "interface",
        "namespace_declaration" => "namespace",
        _ => "unknown",
    };

    let name = extract_name(node, source)?;
    let signature = extract_signature(node, source).unwrap_or_else(|| name.clone());

    Some(ScopeInfo {
        name,
        signature,
        kind: kind.to_string(),
    })
}

/// Extract name from a node
fn extract_name(node: Node, source: &str) -> Option<String> {
    // Try common name field names
    for field_name in &["name", "property", "field"] {
        if let Some(name_node) = node.child_by_field_name(field_name) {
            if let Ok(text) = name_node.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }

    // Fallback: look for identifier child
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().contains("identifier") {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(text.to_string());
            }
        }
    }

    None
}

/// Extract signature from a node
fn extract_signature(node: Node, source: &str) -> Option<String> {
    // For functions, methods, and constructors, try to get the full signature
    if node.kind().contains("function")
        || node.kind() == "method_definition"
        || node.kind() == "method_declaration"
        || node.kind() == "constructor_declaration"
    {
        // Try to get parameters
        if let Some(params_node) = node.child_by_field_name("parameters") {
            if let Ok(params_text) = params_node.utf8_text(source.as_bytes()) {
                let name = extract_name(node, source).unwrap_or_else(|| "unknown".to_string());

                // Try to get return type
                if let Some(return_node) = node.child_by_field_name("return_type") {
                    if let Ok(return_text) = return_node.utf8_text(source.as_bytes()) {
                        return Some(format!("{}{} {}", name, params_text, return_text));
                    }
                }

                // For C#/Java, try "type" field instead
                if let Some(type_node) = node.child_by_field_name("type") {
                    if let Ok(type_text) = type_node.utf8_text(source.as_bytes()) {
                        return Some(format!("{} {}{}", type_text, name, params_text));
                    }
                }

                return Some(format!("{}{}", name, params_text));
            }
        }
    }

    // Fallback: just return the name
    extract_name(node, source)
}

fn abbreviate_kind(kind: &str) -> &'static str {
    match kind {
        "function" => "fn",
        "method" => "m",
        "class" => "c",
        "struct" => "s",
        "trait" => "t",
        "impl" => "i",
        "enum" => "e",
        "module" => "mod",
        "namespace" => "ns",
        "interface" => "iface",
        "constructor" => "ctor",
        "property" => "prop",
        _ => "u",
    }
}
