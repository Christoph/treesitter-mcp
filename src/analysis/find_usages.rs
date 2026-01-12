//! Find Usages Tool
//!
//! Searches for all usages of a symbol across files.
//!
//! Breaking schema change (v1):
//! ```json
//! {
//!   "sym": "parse",
//!   "h": "file|line|col|type|context",
//!   "u": "src/main.rs|42|10|call|let x = parse(input)\n..."
//! }
//! ```

use std::fs;
use std::io;
use std::path::Path;

use serde_json::json;
use serde_json::Value;
use tiktoken_rs::cl100k_base;
use tree_sitter::{Node, Tree};

use crate::analysis::path_utils;
use crate::common::budget;
use crate::common::budget::BudgetTracker;
use crate::common::compact::CompactOutput;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code};

#[derive(Debug, Clone)]
struct UsageRow {
    file: String,
    line: usize,
    column: usize,
    usage_type: String,
    context: String,
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
    let max_tokens = arguments["max_tokens"].as_u64().map(|v| v as usize);

    log::info!("Finding usages of '{symbol}' in: {path_str}");

    let path = Path::new(path_str);
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Path does not exist: {path_str}"),
        ));
    }

    let mut usages: Vec<UsageRow> = Vec::new();
    let mut context_budget = ContextBudget::new(max_context_lines);

    if path.is_file() {
        let _ = search_file(
            path,
            symbol,
            context_lines,
            &mut context_budget,
            &mut usages,
        )?;
    } else if path.is_dir() {
        let _ = search_directory(
            path,
            symbol,
            context_lines,
            &mut context_budget,
            &mut usages,
        )?;
    }

    // Convert all file paths to relative paths
    for usage in &mut usages {
        usage.file = path_utils::to_relative_path(&usage.file);
    }

    let header = "file|line|col|type|context";

    let (rows, truncated_by_budget) = build_rows_with_budget(
        &usages,
        header,
        max_tokens.unwrap_or(usize::MAX),
        max_tokens.is_some(),
    )?;

    let mut result = json!({
        "sym": symbol,
        "h": header,
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

fn build_rows_with_budget(
    usages: &[UsageRow],
    header: &str,
    max_tokens: usize,
    enforce: bool,
) -> Result<(String, bool), io::Error> {
    if !enforce {
        return Ok((usages_to_rows(usages, header), false));
    }

    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;

    // 10% buffer for conservative estimate.
    let mut tracker = BudgetTracker::new((max_tokens * 9) / 10);

    let mut kept: Vec<UsageRow> = Vec::new();
    for usage in usages {
        // Estimate without serialization (conservative).
        let line = usage.line.to_string();
        let column = usage.column.to_string();
        let total_chars = usage.file.len()
            + line.len()
            + column.len()
            + usage.usage_type.len()
            + usage.context.len()
            + 4;

        let estimated = budget::estimate_symbol_tokens(total_chars);
        if !tracker.add(estimated) {
            break;
        }
        kept.push(usage.clone());
    }

    let mut truncated = kept.len() < usages.len();

    // Hard enforcement by truncating rows from the end until we fit.
    loop {
        let candidate_rows = usages_to_rows(&kept, header);
        let candidate_json = serde_json::to_string(&json!({
            "sym": "_",
            "h": header,
            "u": candidate_rows,
        }))
        .map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize result to JSON: {e}"),
            )
        })?;

        if bpe.encode_with_special_tokens(&candidate_json).len() <= max_tokens {
            return Ok((candidate_rows, truncated));
        }

        if kept.pop().is_none() {
            return Ok((String::new(), true));
        }
        truncated = true;
    }
}

fn usages_to_rows(usages: &[UsageRow], header: &str) -> String {
    let mut output = CompactOutput::new(header);

    for usage in usages {
        let line = usage.line.to_string();
        let column = usage.column.to_string();

        output.add_row(&[
            &usage.file,
            &line,
            &column,
            &usage.usage_type,
            &usage.context,
        ]);
    }

    output.rows_string()
}

struct ContextBudget {
    max_total_lines: Option<u32>,
    used_lines: u32,
}

impl ContextBudget {
    fn new(max_total_lines: Option<u32>) -> Self {
        Self {
            max_total_lines,
            used_lines: 0,
        }
    }

    fn can_add_lines(&self, additional: u32) -> bool {
        match self.max_total_lines {
            None => true,
            Some(max) => self.used_lines + additional <= max,
        }
    }

    fn add_lines(&mut self, additional: u32) -> bool {
        if self.can_add_lines(additional) {
            self.used_lines += additional;
            true
        } else {
            false
        }
    }

    fn max_is_zero(&self) -> bool {
        matches!(self.max_total_lines, Some(0))
    }
}

fn search_directory(
    dir: &Path,
    symbol: &str,
    context_lines: u32,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> Result<bool, io::Error> {
    let entries = fs::read_dir(dir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read directory {}: {e}", dir.display()),
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
                continue;
            }
        }

        if path.is_file() {
            if detect_language(&path).is_ok()
                && !search_file(&path, symbol, context_lines, budget, usages)?
            {
                return Ok(false);
            }
        } else if path.is_dir() && !search_directory(&path, symbol, context_lines, budget, usages)?
        {
            return Ok(false);
        }
    }

    Ok(true)
}

fn search_file(
    path: &Path,
    symbol: &str,
    context_lines: u32,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> Result<bool, io::Error> {
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

    Ok(find_identifiers(
        &tree,
        &source,
        symbol,
        path,
        context_lines,
        budget,
        usages,
    ))
}

fn find_identifiers(
    tree: &Tree,
    source: &str,
    symbol: &str,
    path: &Path,
    context_lines: u32,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> bool {
    let root = tree.root_node();
    let mut cursor = root.walk();
    visit_node(
        &mut cursor,
        source,
        symbol,
        path,
        context_lines,
        budget,
        usages,
    )
}

fn visit_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    symbol: &str,
    path: &Path,
    context_lines: u32,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> bool {
    let node = cursor.node();

    if node.kind() == "identifier" || node.kind().ends_with("_identifier") {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if text == symbol {
                let start_pos = node.start_position();
                let usage_type = classify_usage_type(&node);

                let context = if budget.max_is_zero() {
                    String::new()
                } else {
                    extract_code_with_context(source, start_pos.row, context_lines)
                };

                let context_line_count = if budget.max_is_zero() {
                    0
                } else {
                    context.lines().count() as u32
                };

                if !budget.add_lines(context_line_count) {
                    return false;
                }

                usages.push(UsageRow {
                    file: path.to_string_lossy().to_string(),
                    line: start_pos.row + 1,
                    column: start_pos.column + 1,
                    usage_type,
                    context,
                });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            if !visit_node(cursor, source, symbol, path, context_lines, budget, usages) {
                cursor.goto_parent();
                return false;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }

    true
}

fn classify_usage_type(node: &Node) -> String {
    if let Some(parent) = node.parent() {
        let parent_kind = parent.kind();

        if parent_kind == "function_item"
            || parent_kind == "function_declaration"
            || parent_kind == "method_definition"
            || parent_kind == "method_declaration"
            || parent_kind == "struct_item"
            || parent_kind == "class_definition"
            || parent_kind == "class_declaration"
            || parent_kind == "enum_item"
            || parent_kind == "interface_declaration"
            || parent_kind == "type_alias_declaration"
        {
            return "definition".to_string();
        }

        if parent_kind == "let_declaration"
            || parent_kind == "const_item"
            || parent_kind == "static_item"
            || parent_kind == "variable_declarator"
            || parent_kind == "lexical_declaration"
        {
            return "definition".to_string();
        }

        if parent_kind == "use_declaration"
            || parent_kind == "import_statement"
            || parent_kind == "import_clause"
            || parent_kind == "import_specifier"
        {
            return "import".to_string();
        }

        if parent_kind == "call_expression"
            || parent_kind == "method_call_expression"
            || parent_kind == "call"
        {
            return "call".to_string();
        }

        if parent_kind == "type_annotation"
            || parent_kind == "type_identifier"
            || parent_kind == "generic_type"
            || parent_kind == "type_arguments"
            || parent_kind == "type_parameter"
        {
            return "type_reference".to_string();
        }

        if let Some(grandparent) = parent.parent() {
            let grandparent_kind = grandparent.kind();

            if grandparent_kind == "let_declaration"
                || grandparent_kind == "const_item"
                || grandparent_kind == "variable_declaration"
            {
                return "definition".to_string();
            }

            if grandparent_kind == "parameter"
                || grandparent_kind == "formal_parameter"
                || grandparent_kind == "return_type"
            {
                return "type_reference".to_string();
            }

            if grandparent_kind == "call_expression" || grandparent_kind == "method_call_expression"
            {
                return "call".to_string();
            }

            if let Some(great_grandparent) = grandparent.parent() {
                let great_grandparent_kind = great_grandparent.kind();

                if great_grandparent_kind == "let_declaration"
                    || great_grandparent_kind == "const_item"
                    || great_grandparent_kind == "variable_declaration"
                {
                    return "definition".to_string();
                }
            }
        }
    }

    "reference".to_string()
}

fn extract_code_with_context(source: &str, line: usize, context_lines: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let context_lines = context_lines as usize;

    let start_line = line.saturating_sub(context_lines);
    let end_line = std::cmp::min(line + context_lines + 1, lines.len());

    lines[start_line..end_line].join("\n")
}
