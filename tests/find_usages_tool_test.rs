mod common;

use serde_json::json;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_find_usages_locates_function_definition() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(usages["sym"], "add");
    assert_eq!(usages["h"], "file|line|col|type|context");

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(rows.len() >= 1);

    let definition = rows
        .iter()
        .find(|r| r.get(3).map(|s| s.as_str()) == Some("definition"));
    assert!(definition.is_some());
}

#[test]
fn test_find_usages_searches_across_multiple_files() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(rows.len() >= 2);

    let files: std::collections::HashSet<_> =
        rows.iter().filter_map(|r| r.first().cloned()).collect();
    assert!(files.len() >= 2);
}

#[test]
fn test_find_usages_includes_surrounding_context_lines() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 5
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(rows.len() >= 1);

    let context = &rows[0][4];
    assert!(context.lines().count() >= 3);
    assert!(
        context.contains("add") || context.contains("pub fn") || context.contains("a + b"),
        "Context should contain actual fixture code"
    );
}

// ============================================================================
// Cross-Language Smoke Tests
// ============================================================================

#[test]
fn test_find_usages_handles_python_references() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(!rows.is_empty());
}

#[test]
fn test_find_usages_handles_javascript_references() {
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(!rows.is_empty());
}

#[test]
fn test_find_usages_handles_typescript_references() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "symbol": "Point",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert!(!rows.is_empty());
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_find_usages_not_found() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "nonexistent_function_xyz",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert_eq!(rows.len(), 0);
}

#[test]
fn test_find_usages_max_context_lines_enforced() {
    let dir_path = common::fixture_dir("rust");
    let unbounded_args = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap(),
        "context_lines": 5
    });
    let bounded_args = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap(),
        "context_lines": 5,
        "max_context_lines": 1
    });

    let unbounded = treesitter_mcp::analysis::find_usages::execute(&unbounded_args).unwrap();
    let bounded = treesitter_mcp::analysis::find_usages::execute(&bounded_args).unwrap();

    let unbounded_text = common::get_result_text(&unbounded);
    let bounded_text = common::get_result_text(&bounded);

    let unbounded_json: serde_json::Value = serde_json::from_str(&unbounded_text).unwrap();
    let bounded_json: serde_json::Value = serde_json::from_str(&bounded_text).unwrap();

    let unbounded_rows = common::helpers::find_usages_rows(&unbounded_json);
    let bounded_rows = common::helpers::find_usages_rows(&bounded_json);

    assert!(!unbounded_rows.is_empty());
    assert!(bounded_rows.len() <= unbounded_rows.len());

    // max_context_lines=1 should keep at most 1 non-empty context line
    if let Some(first) = bounded_rows.first() {
        let context = &first[4];
        assert!(context.lines().count() <= 1);
    }
}

#[test]
fn test_find_usages_max_tokens_enforced() {
    let dir_path = common::fixture_dir("rust");
    let unbounded_args = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap()
    });
    // Set a very small token limit that should force truncation
    let bounded_args = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap(),
        "max_tokens": 10
    });

    let unbounded = treesitter_mcp::analysis::find_usages::execute(&unbounded_args).unwrap();
    let bounded = treesitter_mcp::analysis::find_usages::execute(&bounded_args).unwrap();

    let unbounded_text = common::get_result_text(&unbounded);
    let bounded_text = common::get_result_text(&bounded);

    // Bounded text should be shorter than unbounded text
    assert!(bounded_text.len() < unbounded_text.len());

    // Verify it's still valid JSON
    let bounded_json: serde_json::Value = serde_json::from_str(&bounded_text).unwrap();

    // Check for truncation indicator in metadata if implemented, or just check that we got results
    // The implementation might drop rows or truncate content
    let rows = common::helpers::find_usages_rows(&bounded_json);
    // Depending on how aggressive 10 tokens is, we might get 0 rows or partial rows
    // Main thing is that it didn't crash and output is smaller
}
