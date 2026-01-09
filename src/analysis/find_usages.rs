//! Find Usages Tool
//!
//! Searches for all usages of a symbol (function, struct, class) across files.
//! Uses tree-sitter to parse and search for identifier nodes.
//! Returns usage locations with code snippets, usage type classification, and AST node information.

use crate::analysis::path_utils;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::Path;
use tree_sitter::{Node, Tree};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    usage_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    node_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
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

    let context_lines = arguments["context_lines"]
        .as_u64()
        .map(|v| v as u32)
        .unwrap_or(3);

    let max_context_lines = arguments["max_context_lines"].as_u64().map(|v| v as u32);

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
        search_file(path, symbol, context_lines, &mut usages)?;
    } else if path.is_dir() {
        search_directory(path, symbol, context_lines, &mut usages)?;
    }

    // Apply context cap if specified
    if let Some(max) = max_context_lines {
        apply_context_cap(&mut usages, context_lines, max);
    }

    // Convert all file paths to relative paths
    for usage in &mut usages {
        usage.file = path_utils::to_relative_path(&usage.file);
    }

    let result = FindUsagesResult {
        symbol: symbol.to_string(),
        usages,
    };

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;
    Ok(CallToolResult::success(result_json))
}

/// Recursively search directory for symbol usages
fn search_directory(
    dir: &Path,
    symbol: &str,
    context_lines: u32,
    usages: &mut Vec<Usage>,
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
            // Only search files with detectable language
            if detect_language(&path).is_ok() {
                let _ = search_file(&path, symbol, context_lines, usages);
            }
        } else if path.is_dir() {
            search_directory(&path, symbol, context_lines, usages)?;
        }
    }

    Ok(())
}

/// Search for symbol usages in a single file
fn search_file(
    path: &Path,
    symbol: &str,
    context_lines: u32,
    usages: &mut Vec<Usage>,
) -> Result<(), io::Error> {
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

    find_identifiers(&tree, &source, symbol, path, context_lines, usages);

    Ok(())
}

/// Find all identifier nodes matching the symbol name
fn find_identifiers(
    tree: &Tree,
    source: &str,
    symbol: &str,
    path: &Path,
    context_lines: u32,
    usages: &mut Vec<Usage>,
) {
    let root = tree.root_node();
    let mut cursor = root.walk();

    visit_node(&mut cursor, source, symbol, path, context_lines, usages);
}

/// Recursively visit nodes to find matching identifiers
fn visit_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    symbol: &str,
    path: &Path,
    context_lines: u32,
    usages: &mut Vec<Usage>,
) {
    let node = cursor.node();

    // Check if this node is an identifier that matches our symbol
    if node.kind() == "identifier" || node.kind().ends_with("_identifier") {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if text == symbol {
                let start_pos = node.start_position();
                let usage_type = classify_usage_type(&node, source);
                let code = extract_code_with_context(source, start_pos.row, context_lines);

                usages.push(Usage {
                    file: path.to_string_lossy().to_string(),
                    line: start_pos.row + 1,
                    column: start_pos.column + 1,
                    usage_type: Some(usage_type),
                    node_type: Some(node.kind().to_string()),
                    code: Some(code),
                });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            visit_node(cursor, source, symbol, path, context_lines, usages);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
}

/// Classify the usage type based on the node and its context
/// Checks parent, grandparent, and great-grandparent nodes for better classification
fn classify_usage_type(node: &Node, _source: &str) -> String {
    // Check parent node to determine usage type
    if let Some(parent) = node.parent() {
        let parent_kind = parent.kind();

        // Definition: function_item, struct_item, class_definition, etc.
        if parent_kind == "function_item"
            || parent_kind == "function_declaration"
            || parent_kind == "method_definition"
            || parent_kind == "method_declaration" // C# methods
            || parent_kind == "struct_item"
            || parent_kind == "class_definition"
            || parent_kind == "class_declaration"
            || parent_kind == "enum_item"
            || parent_kind == "interface_declaration"
            || parent_kind == "type_alias_declaration"
        {
            return "definition".to_string();
        }

        // Variable declarations: let, const, var
        if parent_kind == "let_declaration"
            || parent_kind == "const_item"
            || parent_kind == "static_item"
            || parent_kind == "variable_declarator"
            || parent_kind == "lexical_declaration"
        {
            return "definition".to_string();
        }

        // Import: use_declaration, import_statement, import_clause
        if parent_kind == "use_declaration"
            || parent_kind == "import_statement"
            || parent_kind == "import_clause"
            || parent_kind == "import_specifier"
        {
            return "import".to_string();
        }

        // Call: call_expression, method_call_expression
        if parent_kind == "call_expression"
            || parent_kind == "method_call_expression"
            || parent_kind == "call"
        {
            return "call".to_string();
        }

        // Type reference: type_annotation, type_identifier, generic_type
        if parent_kind == "type_annotation"
            || parent_kind == "type_identifier"
            || parent_kind == "generic_type"
            || parent_kind == "type_arguments"
            || parent_kind == "type_parameter"
        {
            return "type_reference".to_string();
        }

        // Check grandparent for more context
        if let Some(grandparent) = parent.parent() {
            let grandparent_kind = grandparent.kind();

            // Variable declarations in grandparent
            if grandparent_kind == "let_declaration"
                || grandparent_kind == "const_item"
                || grandparent_kind == "variable_declaration"
            {
                return "definition".to_string();
            }

            // Type reference in function parameters or return types
            if grandparent_kind == "parameter"
                || grandparent_kind == "formal_parameter"
                || grandparent_kind == "return_type"
            {
                return "type_reference".to_string();
            }

            // Call in method chain or nested calls
            if grandparent_kind == "call_expression" || grandparent_kind == "method_call_expression"
            {
                return "call".to_string();
            }

            // Check great-grandparent for destructuring patterns
            if let Some(great_grandparent) = grandparent.parent() {
                let great_grandparent_kind = great_grandparent.kind();

                // Destructuring in variable declarations
                if great_grandparent_kind == "let_declaration"
                    || great_grandparent_kind == "const_item"
                    || great_grandparent_kind == "variable_declaration"
                {
                    return "definition".to_string();
                }
            }
        }
    }

    // Default to reference for other cases
    "reference".to_string()
}

/// Extract code snippet with context lines around the target line
fn extract_code_with_context(source: &str, line: usize, context_lines: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let context_lines = context_lines as usize;

    let start_line = line.saturating_sub(context_lines);

    let end_line = std::cmp::min(line + context_lines + 1, lines.len());

    lines[start_line..end_line].join("\n")
}

/// Apply context line cap by truncating usages if total exceeds max
fn apply_context_cap(usages: &mut Vec<Usage>, context_per_usage: u32, max_total: u32) {
    if usages.is_empty() {
        return;
    }

    // Special case: if max is 0, remove all code but keep metadata
    if max_total == 0 {
        for usage in usages.iter_mut() {
            usage.code = None;
        }
        return;
    }

    // Calculate lines per usage (context before + line + context after)
    let lines_per_usage = (context_per_usage * 2) + 1;

    // Calculate max usages we can include
    let max_usages = (max_total / lines_per_usage).max(1) as usize;

    // Truncate if we exceed the limit
    if usages.len() > max_usages {
        usages.truncate(max_usages);
    }
}
