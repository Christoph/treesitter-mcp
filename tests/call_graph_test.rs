mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_call_graph_returns_depth_one_callers_and_callees() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let file_path = src.join("lib.rs");
    fs::write(&file_path, rust_call_graph_fixture()).unwrap();

    let result = treesitter_mcp::analysis::call_graph::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "build_report",
        "direction": "both",
        "depth": 1,
        "max_tokens": 4000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["sym"], "build_report");
    assert_eq!(output["h"], "direction|symbol|file|line|scope|depth");

    let rows = common::helpers::parse_compact_rows(output["edges"].as_str().unwrap());
    assert!(has_edge(&rows, "callee", "normalize_input", 1));
    assert!(has_edge(&rows, "callee", "format_report", 1));
    assert!(has_edge(&rows, "caller", "render_page", 1));
    assert!(!has_edge(&rows, "callee", "unused_helper", 1));
}

#[test]
fn test_call_graph_depth_two_includes_transitive_callees() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let file_path = src.join("lib.rs");
    fs::write(&file_path, rust_call_graph_fixture()).unwrap();

    let result = treesitter_mcp::analysis::call_graph::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "build_report",
        "direction": "callees",
        "depth": 2,
        "max_tokens": 4000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = common::helpers::parse_compact_rows(output["edges"].as_str().unwrap());

    assert!(has_edge(&rows, "callee", "trim_value", 2));
}

#[test]
fn test_call_graph_recursive_function_does_not_loop_forever() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let file_path = src.join("lib.rs");
    fs::write(&file_path, rust_call_graph_fixture()).unwrap();

    let result = treesitter_mcp::analysis::call_graph::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "recursive",
        "direction": "callees",
        "depth": 3,
        "max_tokens": 4000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = common::helpers::parse_compact_rows(output["edges"].as_str().unwrap());

    assert_eq!(
        rows.iter()
            .filter(|row| row.get(1).map(String::as_str) == Some("recursive"))
            .count(),
        1
    );
}

fn has_edge(rows: &[Vec<String>], direction: &str, symbol: &str, depth: usize) -> bool {
    rows.iter().any(|row| {
        row.first().map(String::as_str) == Some(direction)
            && row.get(1).map(String::as_str) == Some(symbol)
            && row.get(5).and_then(|value| value.parse::<usize>().ok()) == Some(depth)
    })
}

fn rust_call_graph_fixture() -> &'static str {
    r#"
pub fn trim_value(value: &str) -> String {
    value.trim().to_string()
}

pub fn normalize_input(value: &str) -> String {
    trim_value(value)
}

pub fn format_report(value: String) -> String {
    format!("report:{value}")
}

pub fn build_report(value: &str) -> String {
    let normalized = normalize_input(value);
    format_report(normalized)
}

pub fn render_page(value: &str) -> String {
    build_report(value)
}

pub fn unused_helper() -> String {
    "unused".to_string()
}

pub fn recursive(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    recursive(value - 1)
}
"#
}
