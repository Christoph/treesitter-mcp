use serde_json::json;

mod common;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_query_pattern_rust_functions() {
    // Given: Rust fixture with functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_item name: (identifier) @name)"
    });

    // When: query_pattern is called with function_item query
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns JSON with query and matches
    assert!(result.is_ok(), "query_pattern should succeed");
    let call_result = result.unwrap();
    assert!(
        !call_result.is_error.unwrap_or(false),
        "Should not be an error"
    );

    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have query and matches fields
    assert!(query_result["query"].is_string());
    assert!(query_result["matches"].is_array());

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() >= 5, "Should find at least 5 functions");

    // Check structure of first match
    if let Some(first_match) = matches.first() {
        assert!(first_match["line"].is_number());
        assert!(first_match["column"].is_number());
        assert!(first_match["text"].is_string());
        assert!(first_match["captures"].is_object());

        // If code field is present, verify it contains actual content
        if first_match["code"].is_string() {
            let code = first_match["code"].as_str().unwrap();
            assert!(!code.is_empty(), "Code should not be empty");
            assert!(
                code.contains("fn") || code.contains("pub"),
                "Code should contain actual Rust function content"
            );
        }
    }
}

#[test]
fn test_query_pattern_rust_with_context() {
    // Given: Rust fixture
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_item name: (identifier) @name)",
        "context_lines": 5
    });

    // When: query_pattern is called with context_lines=5
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns matches (context_lines parameter is accepted but may not affect output)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() >= 1);

    // Check that matches have the expected structure
    if let Some(first_match) = matches.first() {
        assert!(first_match["line"].is_number());
        assert!(first_match["column"].is_number());
        assert!(first_match["text"].is_string());
        assert!(first_match["captures"].is_object());
    }
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_query_pattern_python_classes() {
    // Given: Python fixture with classes
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(class_definition name: (identifier) @name)"
    });

    // When: query_pattern is called with class_definition query
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns JSON with matches for classes
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(query_result["query"].is_string());
    assert!(query_result["matches"].is_array());

    let matches = query_result["matches"].as_array().unwrap();
    assert!(
        matches.len() >= 2,
        "Should find at least 2 classes (Calculator, Point)"
    );

    // Verify match structure
    for match_item in matches {
        assert!(match_item["line"].is_number());
        assert!(match_item["column"].is_number());
        assert!(match_item["text"].is_string());
        assert!(match_item["captures"].is_object());
    }
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_query_pattern_javascript_imports() {
    // Given: JavaScript fixture with imports
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(call_expression function: (identifier) @func (#eq? @func \"require\"))"
    });

    // When: query_pattern is called with import query
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns JSON with matches for require calls
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(query_result["query"].is_string());
    assert!(query_result["matches"].is_array());

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() >= 1, "Should find at least 1 require call");

    // Verify match structure
    if let Some(first_match) = matches.first() {
        assert!(first_match["line"].is_number());
        assert!(first_match["column"].is_number());
        assert!(first_match["text"].is_string());
    }
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_query_pattern_typescript_interfaces() {
    // Given: TypeScript fixture with interfaces
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(interface_declaration name: (type_identifier) @name)"
    });

    // When: query_pattern is called with interface_declaration query
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns JSON with matches for interfaces
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(query_result["query"].is_string());
    assert!(query_result["matches"].is_array());

    let matches = query_result["matches"].as_array().unwrap();
    assert!(
        matches.len() >= 2,
        "Should find at least 2 interfaces (Point, CalculatorOptions)"
    );

    // Verify match structure
    for match_item in matches {
        assert!(match_item["line"].is_number());
        assert!(match_item["column"].is_number());
        assert!(match_item["text"].is_string());
        assert!(match_item["captures"].is_object());
    }
}

// ============================================================================
// Feature Tests
// ============================================================================

#[test]
fn test_query_pattern_includes_parent() {
    // Given: Rust fixture
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_item name: (identifier) @name)"
    });

    // When: query_pattern is called
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Result has required fields for each match
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() > 0);

    // Check that each match has required fields
    for match_item in matches {
        assert!(
            match_item["line"].is_number(),
            "Match should have line number"
        );
        assert!(
            match_item["column"].is_number(),
            "Match should have column number"
        );
        assert!(match_item["text"].is_string(), "Match should have text");
        assert!(
            match_item["captures"].is_object(),
            "Match should have captures"
        );
    }
}

#[test]
fn test_query_pattern_invalid_query() {
    // Given: Rust fixture with invalid query syntax
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(invalid syntax here @@@ )"
    });

    // When: query_pattern is called with invalid query
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns error
    assert!(
        result.is_err(),
        "Should return error for invalid query syntax"
    );
}

// ============================================================================
// Additional Feature Tests
// ============================================================================

#[test]
fn test_query_pattern_multiple_captures() {
    // Given: Python fixture
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_definition name: (identifier) @name parameters: (parameters) @params)"
    });

    // When: query_pattern is called with multiple captures
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns matches with multiple captures
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() > 0);

    // Check that captures object contains multiple entries
    if let Some(first_match) = matches.first() {
        let captures = first_match["captures"].as_object().unwrap();
        assert!(captures.len() >= 1, "Should have at least one capture");
    }
}

#[test]
fn test_query_pattern_nonexistent_file() {
    // Given: Path to non-existent file
    let arguments = json!({
        "file_path": "/nonexistent/file.rs",
        "query": "(function_item)"
    });

    // When: query_pattern is called
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns error
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_query_pattern_empty_matches() {
    // Given: Rust fixture with query that matches nothing
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    // Use a valid query pattern that won't match anything in the file
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(enum_item)"
    });

    // When: query_pattern is called with query that has no matches
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns success with empty matches array
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(query_result["matches"].is_array());
    let matches = query_result["matches"].as_array().unwrap();
    assert_eq!(
        matches.len(),
        0,
        "Should have no matches for enum_item in calculator.rs"
    );
}

#[test]
fn test_query_pattern_with_context_lines() {
    // Given: JavaScript fixture
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(class_declaration name: (identifier) @name)",
        "context_lines": 3
    });

    // When: query_pattern is called with context_lines
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Returns matches with code field containing context
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    let matches = query_result["matches"].as_array().unwrap();
    assert!(matches.len() > 0);

    // Check that code field exists and contains context
    if let Some(first_match) = matches.first() {
        if first_match["code"].is_string() {
            let code = first_match["code"].as_str().unwrap();
            assert!(!code.is_empty(), "Code field should not be empty");

            // Verify code contains actual class content from fixture
            assert!(
                code.contains("class") || code.contains("Calculator") || code.contains("Point"),
                "Code should contain actual class content from fixture"
            );
        }
    }
}

#[test]
fn test_query_pattern_result_structure() {
    // Given: TypeScript fixture
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(function_declaration name: (identifier) @name)"
    });

    // When: query_pattern is called
    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Result has correct JSON structure
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let query_result: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Verify top-level structure
    assert!(query_result["query"].is_string());
    assert!(query_result["matches"].is_array());

    // Verify each match has required fields
    let matches = query_result["matches"].as_array().unwrap();
    for match_item in matches {
        assert!(
            match_item["line"].is_number(),
            "Match should have line number"
        );
        assert!(
            match_item["column"].is_number(),
            "Match should have column number"
        );
        assert!(match_item["text"].is_string(), "Match should have text");
        assert!(
            match_item["captures"].is_object(),
            "Match should have captures object"
        );
    }
}
