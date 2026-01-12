use serde_json::json;

mod common;

fn rows(json: &serde_json::Value) -> Vec<Vec<String>> {
    common::helpers::parse_compact_rows(json["m"].as_str().unwrap_or(""))
}

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_query_pattern_rust_functions() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let query = "(function_item name: (identifier) @name)";
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": query
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok(), "query_pattern should succeed");

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(query_result["q"], query);
    assert_eq!(query_result["h"], "file|line|col|text");

    let matches = rows(&query_result);
    assert!(matches.len() >= 5, "Should find at least 5 functions");

    if let Some(first) = matches.first() {
        assert_eq!(first.len(), 4);
        assert!(first[0].contains("calculator.rs"));
        assert!(!first[3].is_empty());
    }
}

#[test]
fn test_query_pattern_rust_with_context_is_accepted() {
    // context_lines is accepted by the tool args in docs; compact output
    // intentionally does not include extra code context.
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_item name: (identifier) @name)",
        "context_lines": 5
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(query_result["m"].is_string());
    assert!(!rows(&query_result).is_empty());
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_query_pattern_python_classes() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(class_definition name: (identifier) @name)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(query_result["h"], "file|line|col|text");
    assert!(rows(&query_result).len() >= 2);
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_query_pattern_javascript_imports() {
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(call_expression function: (identifier) @func (#eq? @func \"require\"))"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(query_result["h"], "file|line|col|text");
    assert!(!rows(&query_result).is_empty());
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_query_pattern_typescript_interfaces() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(interface_declaration name: (type_identifier) @name)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(query_result["h"], "file|line|col|text");
    assert!(rows(&query_result).len() >= 2);
}

// ============================================================================
// Feature Tests
// ============================================================================

#[test]
fn test_query_pattern_result_structure() {
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_declaration name: (identifier) @name)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(query_result["q"].is_string());
    assert_eq!(query_result["h"], "file|line|col|text");
    assert!(query_result["m"].is_string());

    for row in rows(&query_result) {
        assert_eq!(row.len(), 4);
    }
}

#[test]
fn test_query_pattern_invalid_query() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(invalid syntax here @@@ )"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(
        result.is_err(),
        "Should return error for invalid query syntax"
    );
}

#[test]
fn test_query_pattern_nonexistent_file() {
    let arguments = json!({
        "file_path": "/nonexistent/file.rs",
        "query": "(function_item)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_query_pattern_empty_matches() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(enum_item)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(rows(&query_result).is_empty());
}

#[test]
fn test_query_pattern_multiple_captures_is_supported() {
    // Compact output intentionally omits the capture map for token efficiency,
    // but the query itself should still execute.
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_definition name: (identifier) @name parameters: (parameters) @params)"
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(query_result["m"].is_string());
}
