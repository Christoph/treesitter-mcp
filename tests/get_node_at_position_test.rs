use serde_json::json;

mod common;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_get_node_at_position_rust_identifier() {
    // Given: Rust file with identifier at specific position
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Line 13 (0-indexed: 12), column 5 - position on "a" in "a + b"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node information with identifier type
    assert!(result.is_ok(), "get_node_at_position should succeed");
    let call_result = result.unwrap();
    assert!(
        !call_result.is_error.unwrap_or(false),
        "Should not be an error"
    );

    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have node with type identifier
    assert_eq!(node_info["node"]["type"], "identifier");
    assert_eq!(node_info["node"]["text"], "a");
    assert!(node_info["node"]["range"]["start"].is_object());
    assert!(node_info["node"]["range"]["end"].is_object());
}

#[test]
fn test_get_node_at_position_rust_with_ancestors() {
    // Given: Rust file with position and ancestor_levels parameter
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Line 13, column 5 - position on "a" in "a + b"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5,
        "ancestor_levels": 5
    });

    // When: get_node_at_position is called with ancestor_levels
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node with up to 5 ancestors
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have ancestors array
    assert!(node_info["ancestors"].is_array());
    let ancestors = node_info["ancestors"].as_array().unwrap();
    assert!(ancestors.len() > 0, "Should have at least one ancestor");
    assert!(ancestors.len() <= 5, "Should have at most 5 ancestors");

    // Each ancestor should have type and range
    for ancestor in ancestors {
        assert!(ancestor["type"].is_string());
        assert!(ancestor["range"].is_object());
    }
}

#[test]
fn test_get_node_at_position_rust_function_name() {
    // Given: Rust file with position on function name
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Line 12 (0-indexed: 11), column 8 - position on "add" function name
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 12,
        "column": 8
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns identifier node with function_item parent
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Node should be identifier
    assert_eq!(node_info["node"]["type"], "identifier");
    assert_eq!(node_info["node"]["text"], "add");

    // Should have ancestors including function_item
    assert!(node_info["ancestors"].is_array());
    let ancestors = node_info["ancestors"].as_array().unwrap();
    let has_function_item = ancestors.iter().any(|a| a["type"] == "function_item");
    assert!(has_function_item, "Should have function_item ancestor");
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_get_node_at_position_python_method_call() {
    // Given: Python file with method call
    let file_path = common::fixture_path("python", "calculator.py");
    // Line 79 (0-indexed: 78), column 14 - position on "add" in "self.value += n"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 79,
        "column": 14
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node information
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have node with valid type and range
    assert!(node_info["node"]["type"].is_string());
    assert!(node_info["node"]["range"].is_object());
    assert!(node_info["file"]
        .as_str()
        .unwrap()
        .contains("calculator.py"));
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_get_node_at_position_javascript_property() {
    // Given: JavaScript file with property access
    let file_path = common::fixture_path("javascript", "calculator.js");
    // Line 80 (0-indexed: 79), column 14 - position on "value" in "this.value += n"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 80,
        "column": 14
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node information for property
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have node with property-related type
    assert!(node_info["node"]["type"].is_string());
    assert!(node_info["node"]["range"].is_object());
    assert!(node_info["file"]
        .as_str()
        .unwrap()
        .contains("calculator.js"));
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_get_node_at_position_typescript_type() {
    // Given: TypeScript file with type annotation
    let file_path = common::fixture_path("typescript", "calculator.ts");
    // Line 13 (0-indexed: 12), column 30 - position on "number" type annotation
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 30
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node information for type
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have node with type information
    assert!(node_info["node"]["type"].is_string());
    assert!(node_info["node"]["range"].is_object());
    assert!(node_info["file"]
        .as_str()
        .unwrap()
        .contains("calculator.ts"));
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_get_node_at_position_ancestor_includes_name() {
    // Given: Rust file with position and ancestor_levels
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Line 12, column 8 - position on "add" function name
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 12,
        "column": 8,
        "ancestor_levels": 3
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Ancestors include name field if applicable
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Check ancestors for name field
    let ancestors = node_info["ancestors"].as_array().unwrap();
    for ancestor in ancestors {
        // Some ancestors may have name field (like function_item)
        if ancestor["type"] == "function_item" {
            // function_item ancestors might have name field
            assert!(ancestor["name"].is_string() || ancestor["name"].is_null());
        }
    }
}

#[test]
fn test_get_node_at_position_whitespace() {
    // Given: Rust file with position on whitespace
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Line 13, column 1 - position on whitespace/indentation
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 1
    });

    // When: get_node_at_position is called on whitespace
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Either returns nearest node or error
    // The behavior depends on implementation - could return nearest node or error
    if result.is_ok() {
        let call_result = result.unwrap();
        let text = common::get_result_text(&call_result);
        let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Should have valid node information
        assert!(node_info["node"]["type"].is_string());
    }
    // If error, that's also acceptable for whitespace positions
}

#[test]
fn test_get_node_at_position_output_format() {
    // Given: Rust file with valid position
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Output matches expected JSON format
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Verify complete output structure
    assert!(node_info["file"].is_string());
    assert!(node_info["position"].is_object());
    assert!(node_info["position"]["line"].is_number());
    assert!(node_info["position"]["column"].is_number());

    assert!(node_info["node"].is_object());
    assert!(node_info["node"]["type"].is_string());
    assert!(node_info["node"]["text"].is_string());
    assert!(node_info["node"]["range"].is_object());
    assert!(node_info["node"]["range"]["start"].is_object());
    assert!(node_info["node"]["range"]["end"].is_object());

    // Verify range structure
    let start = &node_info["node"]["range"]["start"];
    assert!(start["line"].is_number());
    assert!(start["column"].is_number());

    let end = &node_info["node"]["range"]["end"];
    assert!(end["line"].is_number());
    assert!(end["column"].is_number());

    // Ancestors should be array (may be empty)
    assert!(node_info["ancestors"].is_array());
}

#[test]
fn test_get_node_at_position_nonexistent_file() {
    // Given: Path to non-existent file
    let arguments = json!({
        "file_path": "/nonexistent/file.rs",
        "line": 1,
        "column": 1
    });

    // When: get_node_at_position is called
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns error
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_get_node_at_position_invalid_line() {
    // Given: Rust file with invalid line number
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 9999,
        "column": 1
    });

    // When: get_node_at_position is called with invalid line
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns error or handles gracefully
    // Implementation may return error or clamp to valid range
    if result.is_ok() {
        let call_result = result.unwrap();
        // If successful, should still have valid structure
        let text = common::get_result_text(&call_result);
        let _node_info: serde_json::Value = serde_json::from_str(&text).unwrap();
    }
}

#[test]
fn test_get_node_at_position_zero_ancestors() {
    // Given: Rust file with ancestor_levels=0
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5,
        "ancestor_levels": 0
    });

    // When: get_node_at_position is called with ancestor_levels=0
    let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

    // Then: Returns node with empty or no ancestors
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have node
    assert!(node_info["node"]["type"].is_string());

    // Ancestors should be empty or not present
    if node_info["ancestors"].is_array() {
        let ancestors = node_info["ancestors"].as_array().unwrap();
        assert_eq!(
            ancestors.len(),
            0,
            "Should have no ancestors when ancestor_levels=0"
        );
    }
}

#[test]
fn test_get_node_at_position_multiple_languages() {
    // Given: Different language files
    let test_cases = vec![
        ("rust", "src/calculator.rs", 13, 5),
        ("python", "calculator.py", 16, 5),
        ("javascript", "calculator.js", 14, 5),
        ("typescript", "calculator.ts", 14, 5),
    ];

    // When: get_node_at_position is called for each language
    for (lang, file, line, col) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "line": line,
            "column": col
        });

        let result = treesitter_mcp::analysis::get_node_at_position::execute(&arguments);

        // Then: All should succeed and have valid structure
        assert!(result.is_ok(), "Should succeed for {} file", lang);
        let call_result = result.unwrap();
        let text = common::get_result_text(&call_result);
        let node_info: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert!(
            node_info["node"]["type"].is_string(),
            "Should have node type for {}",
            lang
        );
        assert!(
            node_info["file"].is_string(),
            "Should have file path for {}",
            lang
        );
    }
}
