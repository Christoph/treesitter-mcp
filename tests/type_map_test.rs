//! Type Map Tests (compact schema)

mod common;

use serde_json::json;

fn parse_type_map(text: &str) -> (String, Vec<Vec<String>>, bool) {
    let out: serde_json::Value = serde_json::from_str(text).unwrap();

    let header = out
        .get("h")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let rows_str = out.get("types").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let truncated = out
        .get("@")
        .and_then(|m| m.get("t"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    (header, rows, truncated)
}

fn row_names(rows: &[Vec<String>]) -> Vec<&str> {
    rows.iter()
        .filter_map(|r| r.first().map(|s| s.as_str()))
        .collect()
}

#[test]
fn test_type_map_compact_header_present() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for TypeScript: {e}"));

    let text = common::get_result_text(&result);
    let (header, rows, _) = parse_type_map(&text);

    assert_eq!(header, "name|kind|file|line|usage_count");
    assert!(!rows.is_empty());
}

#[test]
fn test_type_map_typescript_contains_expected_types() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for TypeScript: {e}"));

    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);

    let names = row_names(&rows);
    assert!(names.contains(&"Point"));
    assert!(names.contains(&"CalculatorOptions"));
    assert!(names.contains(&"OperationResult"));
}

#[test]
fn test_type_map_python_contains_expected_types() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Python: {e}"));

    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);

    let names = row_names(&rows);
    assert!(names.contains(&"Calculator"));
    assert!(names.contains(&"Point"));
    assert!(names.contains(&"LineSegment"));
}

#[test]
fn test_type_map_directory_scan_returns_rows() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for directory: {e}"));

    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);

    assert!(!rows.is_empty());
}

#[test]
fn test_type_map_filters_by_pattern_name() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "pattern": "Point",
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed with pattern: {e}"));

    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);

    assert!(!rows.is_empty());
    for name in row_names(&rows) {
        assert!(name.contains("Point"));
    }
}

#[test]
fn test_type_map_respects_limit_and_offset() {
    let dir_path = common::fixture_dir("typescript");

    let args_limit = json!({
        "path": dir_path.to_str().unwrap(),
        "limit": 2,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&args_limit).unwrap();
    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);
    assert!(rows.len() <= 2);

    let args_offset = json!({
        "path": dir_path.to_str().unwrap(),
        "offset": 1,
        "limit": 1,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&args_offset).unwrap();
    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);
    assert!(rows.len() <= 1);
}

#[test]
fn test_type_map_rows_have_positive_usage_count() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "limit": 50,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let (_header, rows, _) = parse_type_map(&text);

    let any_row = rows.first().expect("Should have at least one row");
    let usage = any_row
        .get(4)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    assert!(usage > 0);
}

#[test]
fn test_type_map_truncation_marker() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "max_tokens": 50
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let (_header, _rows, truncated) = parse_type_map(&text);

    assert!(truncated);
}
