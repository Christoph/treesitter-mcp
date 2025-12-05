mod common;

use serde_json::json;

// ============================================================================
// Rust Tests
// ============================================================================

/// Test that a position inside a function returns the function as innermost context
#[test]
fn test_get_context_rust_inside_function() {
    // Given: Rust file with function at line 12-14
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_context is called for position inside add function
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with function as innermost
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(context["file"], file_path.to_str().unwrap());
    assert_eq!(context["position"]["line"], 13);
    assert_eq!(context["position"]["column"], 5);

    let contexts = context["contexts"].as_array().unwrap();
    assert!(!contexts.is_empty());

    // Innermost context should be the function
    let innermost = &contexts[0];
    assert_eq!(innermost["type"], "function_item");
    assert_eq!(innermost["name"], "add");
    assert!(innermost["signature"]
        .as_str()
        .unwrap()
        .contains("pub fn add"));
    assert!(innermost["code"].is_string());
    let code = innermost["code"].as_str().unwrap();
    assert!(!code.is_empty());

    // Verify code contains actual implementation from fixture
    assert!(
        code.contains("a + b"),
        "Code should contain actual implementation"
    );
    assert!(
        code.contains("pub fn add"),
        "Code should contain function signature"
    );
}

/// Test that a position inside impl block method returns method then impl
#[test]
fn test_get_context_rust_inside_impl() {
    // Given: Rust file with impl block (line 18 is inside the new() method)
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 18,
        "column": 8
    });

    // When: get_context is called for position inside impl method
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with method as innermost, then impl block
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the method
    let method = &contexts[0];
    assert_eq!(method["type"], "function_item");
    assert_eq!(method["name"], "new");

    // Second context should be the impl block
    let impl_block = &contexts[1];
    assert_eq!(impl_block["type"], "impl_item");
    assert_eq!(impl_block["name"], "Calculator");
}

/// Test that a position inside a closure returns closure then function
#[test]
fn test_get_context_rust_nested_closure() {
    // Given: Rust file with nested closure at line 48
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 48,
        "column": 30
    });

    // When: get_context is called for position inside closure
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with closure as innermost, then function
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the closure
    let closure = &contexts[0];
    assert_eq!(closure["type"], "closure_expression");

    // Second context should be the function
    let function = &contexts[1];
    assert_eq!(function["type"], "function_item");
    assert_eq!(function["name"], "apply_operation");
}

// ============================================================================
// Python Tests
// ============================================================================

/// Test that a position inside a class method returns method then class
#[test]
fn test_get_context_python_inside_method() {
    // Given: Python file with class method at line 79
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 79,
        "column": 8
    });

    // When: get_context is called for position inside method
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with method as innermost, then class
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the method
    let method = &contexts[0];
    assert_eq!(method["type"], "function_definition");
    assert_eq!(method["name"], "add");

    // Second context should be the class
    let class = &contexts[1];
    assert_eq!(class["type"], "class_declaration");
    assert_eq!(class["name"], "Calculator");
}

// ============================================================================
// Code Content Verification Tests
// ============================================================================

#[test]
fn test_get_context_code_matches_fixture_exactly() {
    // Given: Rust file with known function at specific position
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Code exactly matches the fixture
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    let function_context = &contexts[0];

    if function_context["code"].is_string() {
        let code = function_context["code"].as_str().unwrap();

        // Verify exact content from fixture
        assert!(
            code.contains("pub fn add(a: i32, b: i32) -> i32"),
            "Should have exact signature"
        );
        assert!(code.contains("a + b"), "Should have exact implementation");

        // Should be complete function, not truncated
        assert!(
            code.trim().starts_with("pub fn") || code.contains("/// Adds"),
            "Should start with function or doc"
        );
        assert!(code.trim().ends_with("}"), "Should end with closing brace");
    }
}

#[test]
fn test_get_context_python_code_includes_docstring() {
    // Given: Python file with function that has docstring
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 16,
        "column": 5
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Code includes the docstring
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    if contexts.len() > 0 {
        let function_context = &contexts[0];

        if function_context["code"].is_string() {
            let code = function_context["code"].as_str().unwrap();

            // Should include docstring
            assert!(
                code.contains("\"\"\"") || code.contains("Adds two numbers"),
                "Should include Python docstring"
            );

            // Should include implementation
            assert!(
                code.contains("return a + b"),
                "Should include implementation"
            );
        }
    }
}

// ============================================================================
// JavaScript Tests
// ============================================================================

/// Test that a position inside an arrow function returns the arrow function
#[test]
fn test_get_context_javascript_arrow_function() {
    // Given: JavaScript file with arrow function at line 55
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 55,
        "column": 30
    });

    // When: get_context is called for position inside arrow function
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with arrow function as innermost
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(!contexts.is_empty());

    // Innermost context should be the arrow function
    let arrow_fn = &contexts[0];
    assert_eq!(arrow_fn["type"], "arrow_function");
    assert!(arrow_fn["code"].is_string());

    // Verify code contains actual arrow function content
    let code = arrow_fn["code"].as_str().unwrap();
    assert!(!code.is_empty(), "Arrow function code should not be empty");
    assert!(
        code.contains("=>") || code.contains("function"),
        "Code should contain function syntax"
    );
}

// ============================================================================
// TypeScript Tests
// ============================================================================

/// Test that a position inside an interface returns the interface
#[test]
fn test_get_context_typescript_interface() {
    // Given: TypeScript file with interface
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 5,
        "column": 10
    });

    // When: get_context is called for position inside interface
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with interface as innermost
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(!contexts.is_empty());

    // Innermost context should be the interface
    let interface = &contexts[0];
    assert_eq!(interface["type"], "interface_declaration");
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test that position at module top level returns only source_file
#[test]
fn test_get_context_at_top_level() {
    // Given: Rust file at top level (line 1)
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 1,
        "column": 0
    });

    // When: get_context is called for top-level position
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns only source_file context
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0]["type"], "source_file");
}

/// Test that innermost context includes full code
#[test]
fn test_get_context_includes_code() {
    // Given: Rust file with function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Innermost context includes full code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    let innermost = &contexts[0];

    // Code should contain the function body
    let code = innermost["code"].as_str().unwrap();
    assert!(
        code.contains("a + b"),
        "Code should contain implementation 'a + b'"
    );
    assert!(code.contains("fn add"), "Code should contain function name");

    // Verify it's the complete function code
    assert!(
        code.lines().count() >= 3,
        "Code should be multi-line function"
    );
}

/// Test that position beyond file bounds returns error or empty
#[test]
fn test_get_context_invalid_position() {
    // Given: Rust file with invalid position (beyond file bounds)
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 9999,
        "column": 0
    });

    // When: get_context is called for invalid position
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns error or empty contexts
    if let Ok(call_result) = result {
        let text = common::get_result_text(&call_result);
        let context: serde_json::Value = serde_json::from_str(&text).unwrap();
        let contexts = context["contexts"].as_array().unwrap();
        // Should be empty or only contain source_file
        assert!(contexts.is_empty() || contexts.len() == 1);
    }
    // If it returns an error, that's also acceptable
}

/// Test that context includes range information
#[test]
fn test_get_context_includes_range() {
    // Given: Rust file with function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Context includes range with start and end positions
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    let innermost = &contexts[0];

    // Should have range with start and end
    assert!(innermost["range"].is_object());
    let range = &innermost["range"];
    assert!(range["start"].is_object());
    assert!(range["end"].is_object());

    // Start should have line and column
    assert!(range["start"]["line"].is_number());
    assert!(range["start"]["column"].is_number());
    assert!(range["end"]["line"].is_number());
    assert!(range["end"]["column"].is_number());
}

/// Test that multiple nested contexts are returned in order
#[test]
fn test_get_context_nested_order() {
    // Given: Rust file with nested closure
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 48,
        "column": 30
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Contexts are returned from innermost to outermost
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First should be innermost (closure)
    assert_eq!(contexts[0]["type"], "closure_expression");

    // Last should be outermost (function)
    assert_eq!(contexts[contexts.len() - 1]["type"], "function_item");
}

/// Test that context works with Python nested functions
#[test]
fn test_get_context_python_nested_function() {
    // Given: Python file with nested function at line 55
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 56,
        "column": 20
    });

    // When: get_context is called for position inside nested function
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with nested function as innermost, then outer function
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the nested function
    let nested_fn = &contexts[0];
    assert_eq!(nested_fn["type"], "function_definition");
    assert_eq!(nested_fn["name"], "formatter");

    // Second context should be the outer function
    let outer_fn = &contexts[1];
    assert_eq!(outer_fn["type"], "function_definition");
    assert_eq!(outer_fn["name"], "apply_operation");
}

/// Test that context signature field is present for functions
#[test]
fn test_get_context_function_signature() {
    // Given: Rust file with function
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 13,
        "column": 5
    });

    // When: get_context is called
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Function context includes signature field
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    let function = &contexts[0];

    assert!(function["signature"].is_string());
    let signature = function["signature"].as_str().unwrap();
    assert!(signature.contains("add"));
    assert!(signature.contains("i32"));
}

/// Test that context works with JavaScript class methods
#[test]
fn test_get_context_javascript_class_method() {
    // Given: JavaScript file with class method at line 80
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 80,
        "column": 8
    });

    // When: get_context is called for position inside class method
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with method as innermost, then class
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the method
    let method = &contexts[0];
    assert_eq!(method["type"], "method_definition");
    assert_eq!(method["name"], "add");

    // Second context should be the class
    let class = &contexts[1];
    assert_eq!(class["type"], "class_declaration");
    assert_eq!(class["name"], "Calculator");
}

/// Test that context works with TypeScript class methods
#[test]
fn test_get_context_typescript_class_method() {
    // Given: TypeScript file with class method at line 87
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 88,
        "column": 8
    });

    // When: get_context is called for position inside class method
    let result = treesitter_mcp::analysis::get_context::execute(&arguments);

    // Then: Returns context with method as innermost, then class
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();

    let contexts = context["contexts"].as_array().unwrap();
    assert!(contexts.len() >= 2);

    // First context should be the method
    let method = &contexts[0];
    assert_eq!(method["type"], "method_definition");
    assert_eq!(method["name"], "add");

    // Second context should be the class
    let class = &contexts[1];
    assert_eq!(class["type"], "class_declaration");
    assert_eq!(class["name"], "Calculator");
}
