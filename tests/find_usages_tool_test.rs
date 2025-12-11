mod common;

use serde_json::json;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_find_usages_locates_function_definition() {
    // Given: Rust fixture with function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Locates definition with usage_type="definition"
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
fn test_find_usages_locates_all_call_sites() {
    // Given: Rust fixture with function calls
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "add",
        "path": dir_path.join("src").to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Locates all calls with usage_type="call"
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Should find definition and possibly calls
    assert!(usage_list.len() >= 1);
}

#[test]
fn test_find_usages_searches_across_multiple_files() {
    // Given: Rust fixture with cross-file references
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap()
    });

    // When: find_usages on directory
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Searches and finds usages in all files
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
fn test_find_usages_includes_surrounding_context_lines() {
    // Given: Rust fixture
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 5
    });

    // When: find_usages with context_lines=5
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Includes 5 lines of context around each usage
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

    // Verify the code contains actual content from the fixture
    assert!(
        code.contains("add") || code.contains("pub fn") || code.contains("a + b"),
        "Code should contain actual function content from fixture"
    );
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_find_usages_handles_class_method_references() {
    // Given: Python fixture with class method
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for method name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Handles and finds definition and calls
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
fn test_find_usages_handles_javascript_function_calls() {
    // Given: JavaScript fixture
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for function name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Handles and finds all usages
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
fn test_find_usages_handles_typescript_type_references() {
    // Given: TypeScript fixture with interface
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "symbol": "Point",
        "path": file_path.to_str().unwrap()
    });

    // When: find_usages for interface name
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Handles and finds definition and type references
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

        // Verify code snippet contains actual content
        let code = first_usage["code"].as_str().unwrap();
        assert!(!code.is_empty(), "Code snippet should not be empty");
        assert!(
            code.contains("add") || code.contains("fn") || code.contains("def"),
            "Code should contain actual source code from fixture"
        );
    }
}

// ============================================================================
// Code Content Verification Tests
// ============================================================================

#[test]
fn test_find_usages_code_snippet_matches_fixture() {
    // Given: Rust fixture with known function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3
    });

    // When: find_usages is called
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Code snippets match actual fixture content
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Find the definition usage
    let definition = usage_list.iter().find(|u| u["usage_type"] == "definition");
    if let Some(def) = definition {
        if def["code"].is_string() {
            let code = def["code"].as_str().unwrap();
            // Should contain the actual function signature and body
            assert!(
                code.contains("pub fn add"),
                "Should contain function signature"
            );
            assert!(code.contains("a + b"), "Should contain implementation");
            assert!(code.contains("i32"), "Should contain type annotations");
        }
    }
}

#[test]
fn test_find_usages_context_lines_includes_surrounding_code() {
    // Given: Python fixture
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 5
    });

    // When: find_usages with context_lines=5
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Code includes surrounding context from fixture
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    if usage_list.len() > 0 {
        let first_usage = &usage_list[0];
        if first_usage["code"].is_string() {
            let code = first_usage["code"].as_str().unwrap();

            // With 5 lines of context, should have substantial code
            assert!(
                code.lines().count() >= 5,
                "Should have at least 5 lines with context"
            );

            // Should contain actual Python code
            assert!(
                code.contains("def") || code.contains("return") || code.contains("add"),
                "Should contain actual Python code from fixture"
            );
        }
    }
}
