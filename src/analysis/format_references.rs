//! Compact formatting for LSP-provided reference locations.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use tree_sitter::{Node, Point, Tree};

use crate::analysis::find_usages::{
    build_rows_with_budget, classify_usage_type, extract_code_with_context, owner_hint,
    scope_for_node, UsageRow, USAGE_HEADER,
};
use crate::analysis::path_utils;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};

#[derive(Debug, Clone)]
struct ReferenceLocation {
    file: PathBuf,
    line: usize,
    column: usize,
}

struct ParsedFile {
    source: String,
    language: Option<Language>,
    tree: Option<Tree>,
}

/// Format LSP-resolved references in the same compact schema as `find_usages`.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let symbol = arguments["symbol"]
        .as_str()
        .or_else(|| arguments["symbol_name"].as_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Missing or invalid 'symbol' argument",
            )
        })?;

    let references = arguments["references"].as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'references' argument",
        )
    })?;

    let context_lines = arguments["context_lines"]
        .as_u64()
        .map(|value| value as u32)
        .unwrap_or(3);
    let max_tokens = arguments["max_tokens"].as_u64().map(|value| value as usize);

    let mut parsed_locations = Vec::new();
    for reference in references {
        parsed_locations.push(parse_reference_location(reference)?);
    }

    let mut file_cache: HashMap<PathBuf, ParsedFile> = HashMap::new();
    let mut usages = Vec::new();

    for location in parsed_locations {
        let parsed = parsed_file(&location.file, &mut file_cache)?;
        let line_index = location.line.saturating_sub(1);
        let column_index = location.column.saturating_sub(1);
        let context = extract_code_with_context(&parsed.source, line_index, context_lines);

        let node = parsed.tree.as_ref().and_then(|tree| {
            identifier_node_at(tree, &parsed.source, line_index, column_index, symbol)
        });

        let (usage_type, scope, owner) = match (node, parsed.language) {
            (Some(node), Some(language)) => (
                classify_usage_type(&node),
                scope_for_node(node, &parsed.source, language),
                owner_hint(node, &parsed.source),
            ),
            _ => ("reference".to_string(), String::new(), None),
        };

        usages.push(UsageRow {
            file: path_utils::to_relative_path(location.file.to_string_lossy().as_ref()),
            line: location.line,
            column: location.column,
            usage_type,
            context,
            scope,
            confidence: "high".to_string(),
            owner_hint: owner,
        });
    }

    usages.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
            .then_with(|| a.usage_type.cmp(&b.usage_type))
            .then_with(|| a.scope.cmp(&b.scope))
    });

    let (rows, truncated_by_budget) = build_rows_with_budget(
        &usages,
        USAGE_HEADER,
        max_tokens.unwrap_or(usize::MAX),
        max_tokens.is_some(),
    )?;

    let mut result = json!({
        "sym": symbol,
        "h": USAGE_HEADER,
        "u": rows,
    });

    if truncated_by_budget {
        result["@"] = json!({"t": true});
    }

    let json_text = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize result to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(json_text))
}

fn parse_reference_location(value: &Value) -> Result<ReferenceLocation, io::Error> {
    let file = value
        .get("file")
        .or_else(|| value.get("file_path"))
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            value
                .get("uri")
                .and_then(Value::as_str)
                .and_then(file_path_from_uri)
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Reference is missing 'file', 'file_path', or 'uri'",
            )
        })?;

    if let Some(start) = value
        .get("range")
        .and_then(|range| range.get("start"))
        .and_then(Value::as_object)
    {
        let line = start.get("line").and_then(Value::as_u64).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "LSP range is missing start.line",
            )
        })? as usize
            + 1;
        let column = start
            .get("character")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "LSP range is missing start.character",
                )
            })? as usize
            + 1;

        return Ok(ReferenceLocation { file, line, column });
    }

    let line = value.get("line").and_then(Value::as_u64).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Reference is missing 1-based 'line'",
        )
    })? as usize;

    let column = value
        .get("col")
        .or_else(|| value.get("column"))
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Reference is missing 1-based 'col' or 'column'",
            )
        })? as usize;

    Ok(ReferenceLocation { file, line, column })
}

fn file_path_from_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(value) = u8::from_str_radix(hex, 16) {
                    output.push(value as char);
                    index += 3;
                    continue;
                }
            }
        }

        output.push(bytes[index] as char);
        index += 1;
    }

    output
}

fn parsed_file<'a>(
    file: &Path,
    cache: &'a mut HashMap<PathBuf, ParsedFile>,
) -> Result<&'a ParsedFile, io::Error> {
    let key = fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    if !cache.contains_key(&key) {
        let source = fs::read_to_string(file).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to read file {}: {e}", file.display()),
            )
        })?;

        let language = detect_language(file).ok();
        let tree = language.and_then(|language| parse_code(&source, language).ok());

        cache.insert(
            key.clone(),
            ParsedFile {
                source,
                language,
                tree,
            },
        );
    }

    cache
        .get(&key)
        .ok_or_else(|| io::Error::other(format!("Failed to cache parsed file {}", file.display())))
}

fn identifier_node_at<'tree>(
    tree: &'tree Tree,
    source: &str,
    line: usize,
    column: usize,
    symbol: &str,
) -> Option<Node<'tree>> {
    if let Some(node) = identifier_node_at_column(tree, line, column, symbol.len()) {
        if identifier_text_matches(node, source, symbol) {
            return Some(node);
        }
    }

    let fallback_column = nearest_symbol_column(source, line, column, symbol)?;
    identifier_node_at_column(tree, line, fallback_column, symbol.len())
}

fn identifier_node_at_column<'tree>(
    tree: &'tree Tree,
    line: usize,
    column: usize,
    symbol_len: usize,
) -> Option<Node<'tree>> {
    let start = Point { row: line, column };
    let end = Point {
        row: line,
        column: column + symbol_len.max(1),
    };

    let root = tree.root_node();
    let node = root.descendant_for_point_range(start, end)?;
    if is_identifier(node) {
        return Some(node);
    }

    identifier_ancestor(node).or_else(|| identifier_descendant_at(root, start))
}

fn identifier_text_matches(node: Node<'_>, source: &str, symbol: &str) -> bool {
    node.utf8_text(source.as_bytes())
        .map(|text| text == symbol)
        .unwrap_or(false)
}

fn nearest_symbol_column(source: &str, line: usize, column: usize, symbol: &str) -> Option<usize> {
    let line_text = source.lines().nth(line)?;
    let mut best: Option<(usize, usize)> = None;

    for (start, _) in line_text.match_indices(symbol) {
        let end = start + symbol.len();
        let distance = if column < start {
            start - column
        } else {
            column.saturating_sub(end)
        };

        match best {
            None => best = Some((start, distance)),
            Some((_, best_distance)) if distance < best_distance => best = Some((start, distance)),
            _ => {}
        }
    }

    best.map(|(start, _)| start)
}

fn identifier_ancestor(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        if is_identifier(node) {
            return Some(node);
        }

        node = node.parent()?;
    }
}

fn identifier_descendant_at<'tree>(node: Node<'tree>, point: Point) -> Option<Node<'tree>> {
    if !contains_point(node, point) {
        return None;
    }

    if is_identifier(node) {
        return Some(node);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = identifier_descendant_at(child, point) {
            return Some(found);
        }
    }

    None
}

fn contains_point(node: Node<'_>, point: Point) -> bool {
    let start = node.start_position();
    let end = node.end_position();

    (point.row > start.row || point.row == start.row && point.column >= start.column)
        && (point.row < end.row || point.row == end.row && point.column <= end.column)
}

fn is_identifier(node: Node<'_>) -> bool {
    node.kind() == "identifier" || node.kind().ends_with("_identifier")
}
