//! Test relevance mapping for changed symbols.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::analysis::find_usages;
use crate::analysis::path_utils;
use crate::analysis::symbol_at_line;
use crate::common::format;
use crate::common::project_files::collect_project_files;
use crate::mcp_types::{CallToolResult, CallToolResultExt};

const TEST_HEADER: &str = "test_file|test_fn|line|relevance";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RelevantTest {
    file: String,
    test_fn: String,
    line: usize,
    relevance: &'static str,
}

/// Return likely relevant tests for a symbol.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;
    let symbol_name = arguments["symbol_name"]
        .as_str()
        .or_else(|| arguments["symbol"].as_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Missing or invalid 'symbol_name' argument",
            )
        })?;

    let target_path = Path::new(file_path);
    let project_root = path_utils::find_project_root(target_path)
        .or_else(|| target_path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));

    let tests = collect_relevant_tests(&project_root, target_path, symbol_name)?;
    let rows = tests
        .iter()
        .map(|test| {
            let line = test.line.to_string();
            format::format_row(&[&test.file, &test.test_fn, &line, test.relevance])
        })
        .collect::<Vec<_>>()
        .join("\n");

    let result = json!({
        "sym": symbol_name,
        "h": TEST_HEADER,
        "tests": rows,
    });
    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize relevant tests result: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}

pub(crate) fn collect_relevant_tests(
    project_root: &Path,
    source_file: &Path,
    symbol_name: &str,
) -> Result<Vec<RelevantTest>, io::Error> {
    let source_stem = source_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let mut tests_by_owner: HashMap<(String, String), RelevantTest> = HashMap::new();
    let mut same_module_candidates = Vec::new();

    for path in collect_project_files(project_root)? {
        if !is_test_file(&path) {
            continue;
        }

        let args = json!({
            "symbol": symbol_name,
            "path": path.to_str().unwrap_or_default(),
            "context_lines": 1,
        });
        let result = find_usages::execute(&args)?;
        let result_text = get_result_text(&result);
        let parsed: Value = serde_json::from_str(&result_text).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse find_usages output: {e}"),
            )
        })?;
        let usage_rows = parsed.get("u").and_then(Value::as_str).unwrap_or("");
        let rel_file = path_utils::to_relative_path(&path.to_string_lossy());

        let mut matched = false;
        for row in usage_rows.lines() {
            let fields = parse_compact_row(row);
            let usage_type = fields.get(3).map(String::as_str).unwrap_or("reference");
            if usage_type == "definition" {
                continue;
            }

            matched = true;
            let line = fields
                .get(1)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1);
            let context = fields.get(4).map(String::as_str).unwrap_or("");
            let test_fn =
                enclosing_test_name(&path, line, fields.get(5).map(String::as_str).unwrap_or(""));
            let relevance = match usage_type {
                "call" => "direct",
                "reference" if looks_like_direct_call(context, symbol_name) => "direct",
                "reference" | "type_reference" | "import" => "indirect",
                _ => "indirect",
            };

            let candidate = RelevantTest {
                file: rel_file.clone(),
                test_fn,
                line,
                relevance,
            };
            let key = (candidate.file.clone(), candidate.test_fn.clone());
            match tests_by_owner.get(&key) {
                Some(existing)
                    if relevance_rank(existing.relevance) <= relevance_rank(relevance) => {}
                _ => {
                    tests_by_owner.insert(key, candidate);
                }
            }
        }

        if !matched && is_same_module_test(path.as_path(), source_stem) {
            same_module_candidates.push(RelevantTest {
                file: rel_file,
                test_fn: String::new(),
                line: 1,
                relevance: "same_module",
            });
        }
    }

    let mut tests = tests_by_owner.into_values().collect::<Vec<_>>();
    let mut seen = tests.iter().cloned().collect::<HashSet<_>>();
    for candidate in same_module_candidates {
        if seen.insert(candidate.clone()) {
            tests.push(candidate);
        }
    }

    tests.sort_by(|a, b| {
        relevance_rank(a.relevance)
            .cmp(&relevance_rank(b.relevance))
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.test_fn.cmp(&b.test_fn))
    });

    Ok(tests)
}

fn relevance_rank(value: &str) -> usize {
    match value {
        "direct" => 0,
        "indirect" => 1,
        "same_module" => 2,
        _ => 3,
    }
}

fn is_test_file(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("/tests/")
        || lower.contains("\\tests\\")
        || lower.contains("_test.")
        || lower.contains(".test.")
        || lower.contains("_spec.")
        || lower.contains(".spec.")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("test_"))
            .unwrap_or(false)
}

fn is_same_module_test(path: &Path, source_stem: &str) -> bool {
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    file_stem.contains(source_stem) || source_stem.contains(file_stem)
}

fn test_name_from_scope(scope: &str) -> String {
    scope
        .rsplit("::")
        .next()
        .unwrap_or(scope)
        .trim()
        .to_string()
}

fn enclosing_test_name(path: &Path, line: usize, fallback_scope: &str) -> String {
    let args = json!({
        "file_path": path.to_str().unwrap_or_default(),
        "line": line,
        "column": 1,
    });
    if let Ok(result) = symbol_at_line::execute(&args) {
        let text = get_result_text(&result);
        if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
            if let Some(symbol) = parsed.get("sym").and_then(Value::as_str) {
                if !symbol.is_empty() {
                    return symbol.to_string();
                }
            }
        }
    }

    test_name_from_scope(fallback_scope)
}

fn looks_like_direct_call(context: &str, symbol_name: &str) -> bool {
    let compact = context.replace(char::is_whitespace, "");
    compact.contains(&format!("{symbol_name}("))
}

fn parse_compact_row(row: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in row.chars() {
        if escaped {
            match ch {
                'n' => current.push('\n'),
                'r' => current.push('\r'),
                '|' => current.push('|'),
                '\\' => current.push('\\'),
                other => {
                    current.push('\\');
                    current.push(other);
                }
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '|' {
            fields.push(std::mem::take(&mut current));
            continue;
        }

        current.push(ch);
    }

    if escaped {
        current.push('\\');
    }
    fields.push(current);
    fields
}

fn get_result_text(result: &CallToolResult) -> String {
    if let Some(first_content) = result.content.first() {
        if let Ok(json_str) = serde_json::to_string(first_content) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&json_str) {
                if let Some(text) = json_val["text"].as_str() {
                    return text.to_string();
                }
            }
        }
    }
    String::new()
}
