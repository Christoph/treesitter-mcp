//! Compact review context for a changed file.

use std::collections::HashSet;
use std::io;

use serde_json::{json, Map, Value};
use tiktoken_rs::cl100k_base;

use crate::analysis::{diff, minimal_edit_context, relevant_tests};
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};

const TEST_HEADER: &str = "symbol|test_file|test_fn|line|relevance";

/// Return a compact review bundle for a changed file.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;
    let compare_to = arguments["compare_to"]
        .as_str()
        .unwrap_or("HEAD")
        .to_string();
    let scope = arguments["scope"].as_str();
    let max_tokens = arguments["max_tokens"]
        .as_u64()
        .map(|value| value as usize)
        .unwrap_or(2000);

    let analysis = diff::analyze_diff(file_path, compare_to.clone())?;
    let parse_diff_result = diff::execute_parse_diff(&json!({
        "file_path": file_path,
        "compare_to": compare_to,
    }))?;
    let parse_diff_text = get_result_text(&parse_diff_result);
    let parse_diff_json: Value = serde_json::from_str(&parse_diff_text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse parse_diff output: {e}"),
        )
    })?;

    let affected_result = diff::execute_affected_by_diff(&json!({
        "file_path": file_path,
        "compare_to": compare_to,
        "scope": scope,
    }))?;
    let affected_text = get_result_text(&affected_result);
    let affected_json: Value = serde_json::from_str(&affected_text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse affected_by_diff output: {e}"),
        )
    })?;

    let mut tests = Vec::new();
    let mut seen_test_rows = HashSet::new();
    let mut ctx = Map::new();
    for change in &analysis.structural_changes {
        if !ctx.contains_key(&change.name) {
            let context_result = minimal_edit_context::execute(&json!({
                "file_path": file_path,
                "symbol_name": change.name,
                "max_tokens": 700,
            }))?;
            let context_text = get_result_text(&context_result);
            let context_json: Value = serde_json::from_str(&context_text).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Failed to parse minimal_edit_context output: {e}"),
                )
            })?;
            ctx.insert(change.name.clone(), context_json);
        }

        let relevant_result = relevant_tests::execute(&json!({
            "file_path": file_path,
            "symbol_name": change.name,
        }))?;
        let relevant_text = get_result_text(&relevant_result);
        let relevant_json: Value = serde_json::from_str(&relevant_text).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse relevant_tests output: {e}"),
            )
        })?;
        let rows = relevant_json
            .get("tests")
            .and_then(Value::as_str)
            .unwrap_or("");
        for row in rows.lines() {
            let fields = parse_compact_row(row);
            let prefixed = format::format_row(&[
                &change.name,
                fields.first().map(String::as_str).unwrap_or(""),
                fields.get(1).map(String::as_str).unwrap_or(""),
                fields.get(2).map(String::as_str).unwrap_or(""),
                fields.get(3).map(String::as_str).unwrap_or(""),
            ]);
            if seen_test_rows.insert(prefixed.clone()) {
                tests.push(prefixed);
            }
        }
    }

    let mut result = json!({
        "p": analysis.file_path,
        "cmp": compare_to,
        "ch": parse_diff_json["h"].as_str().unwrap_or("type|name|line|change"),
        "changes": parse_diff_json["changes"].as_str().unwrap_or(""),
        "ah": affected_json["h"].as_str().unwrap_or("symbol|change|file|line|risk"),
        "affected": affected_json["affected"].as_str().unwrap_or(""),
        "th": TEST_HEADER,
        "tests": tests.join("\n"),
        "ctx": ctx,
    });

    enforce_budget(&mut result, max_tokens)?;

    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize review_context result: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}

fn enforce_budget(result: &mut Value, max_tokens: usize) -> Result<(), io::Error> {
    let bpe = cl100k_base()
        .map_err(|e| io::Error::other(format!("Failed to initialize tiktoken tokenizer: {e}")))?;
    let mut truncated = false;

    loop {
        let candidate = serde_json::to_string(result).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to serialize review_context candidate: {e}"),
            )
        })?;
        if bpe.encode_with_special_tokens(&candidate).len() <= max_tokens {
            if truncated {
                result["@"] = json!({"t": true});
            }
            return Ok(());
        }

        if trim_context(result)
            || trim_multiline_field(result, "tests")
            || trim_multiline_field(result, "affected")
            || trim_multiline_field(result, "changes")
        {
            truncated = true;
            continue;
        }

        result["ctx"] = json!({});
        result["tests"] = json!("");
        result["affected"] = json!("");
        result["changes"] = json!("");
        result["@"] = json!({"t": true});
        return Ok(());
    }
}

fn trim_context(result: &mut Value) -> bool {
    let Some(ctx) = result.get_mut("ctx").and_then(Value::as_object_mut) else {
        return false;
    };
    let Some(last_key) = ctx.keys().next_back().cloned() else {
        return false;
    };
    ctx.remove(&last_key).is_some()
}

fn trim_multiline_field(result: &mut Value, field: &str) -> bool {
    let Some(value) = result.get(field).and_then(Value::as_str) else {
        return false;
    };
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return false;
    }
    lines.pop();
    *result.get_mut(field).unwrap() = json!(lines.join("\n"));
    true
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
