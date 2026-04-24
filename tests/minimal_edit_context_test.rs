mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_minimal_edit_context_keeps_only_relevant_same_file_deps_and_imports() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("workflow.ts");
    fs::write(&file_path, large_typescript_fixture()).unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "buildSummary",
        "max_tokens": 4000
    });

    let result = treesitter_mcp::analysis::minimal_edit_context::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(json["sym"], "buildSummary");
    assert_eq!(json["h"], "name|line|sig|code");
    assert!(json["target"].as_str().unwrap().contains("buildSummary"));
    assert!(json["target"].as_str().unwrap().contains("normalizeInput"));

    let dep_rows = common::helpers::parse_compact_rows(json["deps"].as_str().unwrap());
    let dep_names: Vec<&str> = dep_rows.iter().map(|row| row[1].as_str()).collect();
    assert!(dep_names.contains(&"normalizeInput"));
    assert!(dep_names.contains(&"formatOutput"));
    assert!(!dep_names.contains(&"unusedHelper7"));

    let import_rows = common::helpers::parse_compact_rows(json["imports"].as_str().unwrap());
    let import_text = import_rows
        .iter()
        .map(|row| row[1].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(import_text.contains("Input"));
    assert!(!import_text.contains("UnusedThing"));

    let type_rows = common::helpers::parse_compact_rows(json["types"].as_str().unwrap());
    let type_names: Vec<&str> = type_rows.iter().map(|row| row[1].as_str()).collect();
    assert!(type_names.contains(&"SummaryResult"));
    assert!(!type_names.contains(&"UnusedLocal"));
}

#[test]
fn test_minimal_edit_context_is_smaller_than_focused_view_code() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("workflow.ts");
    fs::write(&file_path, large_typescript_fixture()).unwrap();

    let minimal_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "buildSummary",
        "max_tokens": 8000
    });
    let minimal = treesitter_mcp::analysis::minimal_edit_context::execute(&minimal_args).unwrap();
    let minimal_text = common::get_result_text(&minimal);
    let minimal_tokens = common::helpers::approx_tokens(&minimal_text);

    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "buildSummary",
        "detail": "full",
        "include_deps": false
    });
    let focused = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused);
    let focused_tokens = common::helpers::approx_tokens(&focused_text);

    assert!(
        minimal_tokens * 3 < focused_tokens,
        "minimal_edit_context should be >3x smaller; got {minimal_tokens} vs {focused_tokens}"
    );
}

#[test]
fn test_minimal_edit_context_includes_project_local_dependency_signatures() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"name":"minimal-edit-fixture"}"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let file_path = src.join("workflow.ts");
    fs::write(
        &file_path,
        r#"
import { externalNormalize, unusedExternal } from "./helpers";

export function buildSummary(input: string): string {
  return externalNormalize(input);
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("helpers.ts"),
        r#"
export function externalNormalize(input: string): string {
  return input.trim();
}

export function unusedExternal(input: string): string {
  return input.toUpperCase();
}
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::minimal_edit_context::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "buildSummary",
        "max_tokens": 4000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();
    let dep_rows = common::helpers::parse_compact_rows(output["deps"].as_str().unwrap());
    let dep_names: Vec<&str> = dep_rows.iter().map(|row| row[1].as_str()).collect();

    assert!(dep_names.contains(&"externalNormalize"));
    assert!(!dep_names.contains(&"unusedExternal"));
}

#[test]
fn test_minimal_edit_context_trims_rows_before_dropping_sections() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("workflow.ts");
    fs::write(&file_path, large_typescript_fixture()).unwrap();

    let result = treesitter_mcp::analysis::minimal_edit_context::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "buildSummary",
        "max_tokens": 180
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["sym"], "buildSummary");
    assert!(output["target"].as_str().unwrap().contains("buildSummary"));
    assert_eq!(output["@"], json!({"t": true}));

    let kept_context_rows = ["imports", "types", "deps"]
        .into_iter()
        .filter_map(|field| output.get(field).and_then(|value| value.as_str()))
        .map(|rows| rows.lines().count())
        .sum::<usize>();
    assert!(
        kept_context_rows > 0,
        "tight budgets should keep at least some partial context instead of dropping every section"
    );
}

#[test]
fn test_minimal_edit_context_comment_mode_leading_prepends_comments_to_target_code() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("workflow.ts");
    fs::write(
        &file_path,
        r#"
// Why: downstream callers expect canonicalized keys.
// Keep this in sync with API cache normalization.
export function buildSummary(input: string): string {
  return input.trim();
}
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::minimal_edit_context::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "buildSummary",
        "comment_mode": "leading",
        "max_tokens": 4000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();
    let target = common::helpers::parse_compact_row(output["target"].as_str().unwrap());
    let code = target.get(3).unwrap();

    assert!(code.starts_with("// Why: downstream callers expect canonicalized keys."));
    assert!(code.contains("function buildSummary"));
}

fn large_typescript_fixture() -> String {
    let mut source = String::from(
        r#"
import { Input } from "./types";
import { UnusedThing } from "./unused";

export interface SummaryResult {
  value: string;
}

export interface UnusedLocal {
  value: number;
}

export function normalizeInput(input: Input): string {
  return String(input.value).trim();
}

export function formatOutput(value: string): string {
  return value.toUpperCase();
}

export function buildSummary(input: Input): SummaryResult {
  const normalized = normalizeInput(input);
  const formatted = formatOutput(normalized);
  return { value: formatted };
}
"#,
    );

    for idx in 0..24 {
        source.push_str(&format!(
            r#"
export function unusedHelper{idx}(
  firstArgument: string,
  secondArgument: number,
  thirdArgument: SummaryResult
): string {{
  return `${{firstArgument}}-${{secondArgument}}-${{thirdArgument.value}}`;
}}
"#
        ));
    }

    source
}
