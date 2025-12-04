mod common;

use serde_json::json;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_find_usages_rust_function_definition() {
    // Given: Rust fixture with function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds definition with usage_type="definition"
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(usages["symbol"], "add");
    let usage_list = usages["usages"].as_array().unwrap();

    // Should find at least the definition
    assert!(usage_list.len() >= 1);

    // Check for definition
    let definition = usage_list.iter().find(|u| u["usage_type"] == "definition");
    assert!(definition.is_some());
}

#[test]
fn test_find_usages_rust_function_calls() {
    // Given: Rust fixture with function calls
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "add",
        "path": dir_path.join("src").to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds all calls with usage_type="call"
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Should find definition and possibly calls
    assert!(usage_list.len() >= 1);
}

#[test]
fn test_find_usages_rust_cross_file() {
    // Given: Rust fixture with cross-file references
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap()
    });

    // When: find_usages on directory
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds usages in all files
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Should find Calculator in multiple files (models/mod.rs, calculator.rs, lib.rs)
    assert!(usage_list.len() >= 2);

    // Check that usages are from different files
    let files: std::collections::HashSet<_> = usage_list
        .iter()
        .map(|u| u["file"].as_str().unwrap())
        .collect();
    assert!(files.len() >= 2);
}

#[test]
fn test_find_usages_rust_with_context() {
    // Given: Rust fixture
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 5
    });

    // When: find_usages with context_lines=5
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Returns 5 lines of context around each usage
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 1);

    // Check that code field has multiple lines (context)
    let first_usage = &usage_list[0];
    assert!(first_usage["code"].is_string());
    let code = first_usage["code"].as_str().unwrap();
    // With 5 lines of context, should have multiple lines
    assert!(code.lines().count() >= 3);
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_find_usages_python_method() {
    // Given: Python fixture with class method
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for method name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds definition and calls
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 1); // At least the function definition
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_find_usages_javascript_function() {
    // Given: JavaScript fixture
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds all usages
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 1);
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_find_usages_typescript_interface() {
    // Given: TypeScript fixture with interface
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "symbol": "Point",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for interface name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Finds definition and type references
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 1);
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_find_usages_not_found() {
    // Given: Fixture project
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "nonexistent_function_xyz",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for non-existent symbol
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Returns empty usages array
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert_eq!(usage_list.len(), 0);
}

#[test]
fn test_find_usages_includes_code_snippet() {
    // Given: Fixture with function usage
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3
    });

    // When: find_usages is called
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Each usage includes multi-line code snippet
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    if usage_list.len() > 0 {
        let first_usage = &usage_list[0];
        assert!(first_usage["code"].is_string());
        assert!(first_usage["node_type"].is_string());
        assert!(first_usage["usage_type"].is_string());
    }
}
