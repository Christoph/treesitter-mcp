//! Find Usages Tool
//!
//! Searches for all usages of a symbol across files.
//!
//! Breaking schema change (v1):
//! ```json
//! {
//!   "sym": "parse",
//!   "h": "file|line|col|type|context|scope|conf|owner",
//!   "u": "src/main.rs|42|10|call|let x = parse(input)|main|low|\n..."
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
use crate::common::project_files::collect_project_files;
use crate::mcp_types::{CallToolResult, CallToolResultExt};
use crate::parser::{detect_language, parse_code, Language};

#[derive(Debug, Clone)]
struct UsageRow {
    file: String,
    line: usize,
    column: usize,
    usage_type: String,
    context: String,
    scope: String,
    confidence: String,
    owner_hint: Option<String>,
}

#[derive(Clone, Copy)]
struct SearchTarget<'a> {
    source: &'a str,
    symbol: &'a str,
    language: Language,
    path: &'a Path,
    context_lines: u32,
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

    assign_confidence(&mut usages);
    usages.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.column.cmp(&b.column))
            .then_with(|| a.usage_type.cmp(&b.usage_type))
            .then_with(|| a.scope.cmp(&b.scope))
    });

    // Convert all file paths to relative paths
    for usage in &mut usages {
        usage.file = path_utils::to_relative_path(&usage.file);
    }

    let header = "file|line|col|type|context|scope|conf|owner";

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
            + usage.scope.len()
            + usage.confidence.len()
            + usage.owner_hint.as_deref().unwrap_or("").len()
            + 7;

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
            &usage.scope,
            &usage.confidence,
            usage.owner_hint.as_deref().unwrap_or(""),
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
    for path in collect_project_files(dir)? {
        if detect_language(&path).is_ok()
            && !search_file(&path, symbol, context_lines, budget, usages)?
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

    let search = SearchTarget {
        source: &source,
        symbol,
        language,
        path,
        context_lines,
    };

    Ok(find_identifiers(&tree, search, budget, usages))
}

fn find_identifiers(
    tree: &Tree,
    search: SearchTarget<'_>,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> bool {
    let root = tree.root_node();
    let mut cursor = root.walk();
    visit_node(&mut cursor, search, budget, usages)
}

fn visit_node(
    cursor: &mut tree_sitter::TreeCursor,
    search: SearchTarget<'_>,
    budget: &mut ContextBudget,
    usages: &mut Vec<UsageRow>,
) -> bool {
    let node = cursor.node();

    if node.kind() == "identifier" || node.kind().ends_with("_identifier") {
        if let Ok(text) = node.utf8_text(search.source.as_bytes()) {
            if text == search.symbol {
                let start_pos = node.start_position();
                let usage_type = classify_usage_type(&node);

                let context = if budget.max_is_zero() {
                    String::new()
                } else {
                    extract_code_with_context(search.source, start_pos.row, search.context_lines)
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
                    file: search.path.to_string_lossy().to_string(),
                    line: start_pos.row + 1,
                    column: start_pos.column + 1,
                    usage_type,
                    context,
                    scope: scope_for_node(node, search.source, search.language),
                    confidence: "low".to_string(),
                    owner_hint: owner_hint(node, search.source),
                });
            }
        }
    }

    if cursor.goto_first_child() {
        loop {
            if !visit_node(cursor, search, budget, usages) {
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

fn scope_for_node(node: Node, source: &str, language: Language) -> String {
    let mut parts = Vec::new();
    let mut current = Some(node);

    while let Some(candidate) = current {
        if is_context_node(candidate.kind(), language) {
            if let Some(name) = extract_scope_name(candidate, source) {
                parts.push(name);
            }
        }
        current = candidate.parent();
    }

    parts.reverse();
    parts.join("::")
}

fn is_context_node(node_type: &str, language: Language) -> bool {
    match language {
        Language::Rust => matches!(
            node_type,
            "function_item" | "impl_item" | "trait_item" | "struct_item" | "enum_item" | "mod_item"
        ),
        Language::Python => matches!(node_type, "function_definition" | "class_definition"),
        Language::JavaScript | Language::TypeScript => matches!(
            node_type,
            "function_declaration"
                | "method_definition"
                | "class_declaration"
                | "arrow_function"
                | "function_expression"
        ),
        Language::Go => matches!(
            node_type,
            "function_declaration" | "method_declaration" | "type_spec"
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
        Language::Html | Language::Css | Language::Swift => false,
    }
}

fn extract_scope_name(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "impl_item" => extract_impl_scope_name(node, source),
        "method_declaration" | "method_definition" => extract_method_name(node, source),
        _ => extract_named_field(node, source),
    }
}

fn extract_impl_scope_name(node: Node, source: &str) -> Option<String> {
    if let Some(type_node) = node.child_by_field_name("type") {
        return type_node
            .utf8_text(source.as_bytes())
            .ok()
            .map(normalize_scope_token);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().contains("type_identifier") || child.kind() == "scoped_type_identifier" {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(normalize_scope_token(text));
            }
        }
    }

    None
}

fn extract_method_name(node: Node, source: &str) -> Option<String> {
    node.child_by_field_name("name")
        .or_else(|| node.child_by_field_name("property"))
        .or_else(|| node.child_by_field_name("field"))
        .and_then(|child| child.utf8_text(source.as_bytes()).ok())
        .map(ToOwned::to_owned)
        .or_else(|| extract_named_field(node, source))
}

fn extract_named_field(node: Node, source: &str) -> Option<String> {
    for field_name in ["name", "property", "field"] {
        if let Some(name_node) = node.child_by_field_name(field_name) {
            if let Ok(text) = name_node.utf8_text(source.as_bytes()) {
                return Some(normalize_scope_token(text));
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind().contains("identifier") {
            if let Ok(text) = child.utf8_text(source.as_bytes()) {
                return Some(normalize_scope_token(text));
            }
        }
    }

    None
}

fn owner_hint(node: Node, source: &str) -> Option<String> {
    let parent = node.parent()?;

    for field_name in [
        "function", "receiver", "object", "value", "argument", "module", "scope",
    ] {
        if let Some(candidate) = parent.child_by_field_name(field_name) {
            if candidate.id() == node.id() {
                continue;
            }
            if let Ok(text) = candidate.utf8_text(source.as_bytes()) {
                if let Some(owner) = normalize_owner_hint(text) {
                    return Some(owner);
                }
            }
        }
    }

    let mut cursor = parent.walk();
    let named_children: Vec<Node> = parent
        .children(&mut cursor)
        .filter(|child| child.is_named())
        .collect();
    let position = named_children
        .iter()
        .position(|child| child.id() == node.id())?;
    let previous = named_children.get(position.checked_sub(1)?)?;
    previous
        .utf8_text(source.as_bytes())
        .ok()
        .and_then(normalize_owner_hint)
}

fn normalize_owner_hint(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_generics = trimmed
        .split('<')
        .next()
        .unwrap_or(trimmed)
        .split('(')
        .next()
        .unwrap_or(trimmed)
        .trim();

    without_generics
        .rsplit([':', '.'])
        .find(|segment| !segment.is_empty())
        .map(normalize_scope_token)
}

fn normalize_scope_token(text: &str) -> String {
    text.trim().trim_matches('&').trim().to_string()
}

fn assign_confidence(usages: &mut [UsageRow]) {
    let definitions: Vec<DefinitionSite> = usages
        .iter()
        .filter(|usage| usage.usage_type == "definition")
        .map(DefinitionSite::from)
        .collect();

    for usage in usages.iter_mut() {
        usage.confidence = confidence_for_usage(usage, &definitions).to_string();
    }
}

#[derive(Clone, Copy)]
enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    fn as_str(self) -> &'static str {
        match self {
            Confidence::High => "high",
            Confidence::Medium => "medium",
            Confidence::Low => "low",
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

struct DefinitionSite {
    file: String,
    scope: String,
    owner: Option<String>,
}

impl DefinitionSite {
    fn from(usage: &UsageRow) -> Self {
        Self {
            file: usage.file.clone(),
            scope: usage.scope.clone(),
            owner: definition_owner(usage),
        }
    }
}

fn confidence_for_usage(usage: &UsageRow, definitions: &[DefinitionSite]) -> Confidence {
    if usage.usage_type == "definition" {
        return Confidence::High;
    }

    if definitions.is_empty() {
        return Confidence::Low;
    }

    if let Some(owner_hint) = usage.owner_hint.as_deref() {
        let owner_matches = definitions
            .iter()
            .filter(|definition| {
                owner_matches(definition.owner.as_deref(), owner_hint, &usage.scope)
            })
            .count();
        if owner_matches == 1 {
            return Confidence::High;
        }
    }

    let same_scope_matches = definitions
        .iter()
        .filter(|definition| scope_aligns(&definition.scope, &usage.scope))
        .count();
    if same_scope_matches == 1 {
        return Confidence::High;
    }

    if definitions
        .iter()
        .any(|definition| definition.file == usage.file)
    {
        return Confidence::Medium;
    }

    Confidence::Low
}

fn owner_matches(definition_owner: Option<&str>, owner_hint: &str, usage_scope: &str) -> bool {
    if owner_hint.eq_ignore_ascii_case("self") || owner_hint.eq_ignore_ascii_case("this") {
        return definition_owner
            .map(|owner| usage_scope.split("::").any(|segment| segment == owner))
            .unwrap_or(false);
    }

    definition_owner
        .map(|owner| owner == owner_hint)
        .unwrap_or(false)
}

fn scope_aligns(definition_scope: &str, usage_scope: &str) -> bool {
    let Some(definition_owner) = scope_owner(definition_scope) else {
        return false;
    };

    usage_scope
        .split("::")
        .any(|segment| segment == definition_owner)
}

fn scope_owner(scope: &str) -> Option<&str> {
    scope.rsplit("::").nth(1)
}

fn definition_owner(usage: &UsageRow) -> Option<String> {
    scope_owner(&usage.scope)
        .map(ToOwned::to_owned)
        .or_else(|| {
            Path::new(&usage.file)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToOwned::to_owned)
        })
}

fn extract_code_with_context(source: &str, line: usize, context_lines: u32) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let context_lines = context_lines as usize;

    let start_line = line.saturating_sub(context_lines);
    let end_line = std::cmp::min(line + context_lines + 1, lines.len());

    lines[start_line..end_line].join("\n")
}
