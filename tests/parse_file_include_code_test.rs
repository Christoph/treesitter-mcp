use serde_json::json;

mod common;

// ============================================================================
// Rust Tests - include_code=false
// ============================================================================

#[test]
fn test_parse_file_include_code_false_omits_function_code() {
    // Given: Rust fixture with functions and include_code=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code field
    assert!(result.is_ok(), "parse_file should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty(), "Should have functions");

    for func in functions {
        // Code should be null or absent
        assert!(
            func["code"].is_null() || !func.as_object().unwrap().contains_key("code"),
            "Function '{}' should NOT have code when include_code=false",
            func["name"]
        );
        // But signature and line should still be present
        assert!(
            func["signature"].is_string(),
            "Function '{}' should have signature",
            func["name"]
        );
        assert!(
            func["line"].is_number(),
            "Function '{}' should have line number",
            func["name"]
        );
    }
}

#[test]
fn test_parse_file_include_code_false_omits_struct_code() {
    // Given: Rust fixture with structs and include_code=false
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Structs should NOT have code field
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let structs = shape["structs"].as_array().unwrap();
    assert!(!structs.is_empty(), "Should have structs");

    for struct_item in structs {
        // Code should be null or absent
        assert!(
            struct_item["code"].is_null() || !struct_item.as_object().unwrap().contains_key("code"),
            "Struct '{}' should NOT have code when include_code=false",
            struct_item["name"]
        );
        // But name and line should still be present
        assert!(struct_item["name"].is_string(), "Struct should have name");
        assert!(
            struct_item["line"].is_number(),
            "Struct should have line number"
        );
    }
}

#[test]
fn test_parse_file_include_code_false_omits_class_code() {
    // Given: Python fixture with classes and include_code=false
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code field
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = shape["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should have classes");

    for class_item in classes {
        // Code should be null or absent
        assert!(
            class_item["code"].is_null() || !class_item.as_object().unwrap().contains_key("code"),
            "Class '{}' should NOT have code when include_code=false",
            class_item["name"]
        );
        // But name and line should still be present
        assert!(class_item["name"].is_string(), "Class should have name");
        assert!(
            class_item["line"].is_number(),
            "Class should have line number"
        );
    }
}

// ============================================================================
// Rust Tests - include_code=true
// ============================================================================

#[test]
fn test_parse_file_include_code_true_includes_code() {
    // Given: Rust fixture with include_code=true
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions SHOULD have code field with content
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty(), "Should have functions");

    // At least one function should have code
    let has_code = functions
        .iter()
        .any(|f| f["code"].is_string() && !f["code"].as_str().unwrap().is_empty());
    assert!(
        has_code,
        "At least one function should have code when include_code=true"
    );

    // Verify specific function has code
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    assert!(add_fn["code"].is_string(), "add function should have code");
    let code = add_fn["code"].as_str().unwrap();
    assert!(
        code.contains("a + b"),
        "Code should contain the actual implementation"
    );
}

// ============================================================================
// Default Behavior Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_defaults_to_true() {
    // Given: Rust fixture WITHOUT include_code parameter (should default to true)
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should include code by default (backward compatible)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty());

    // At least one function should have code (default behavior)
    let has_code = functions
        .iter()
        .any(|f| f["code"].is_string() && !f["code"].as_str().unwrap().is_empty());
    assert!(
        has_code,
        "Should include code by default for backward compatibility"
    );
}

// ============================================================================
// Documentation Preservation Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_false_preserves_docs() {
    // Given: Rust fixture with doc comments and include_code=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Doc comments should still be present even without code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    // Should have doc comment
    assert!(
        add_fn["doc"].is_string(),
        "Should preserve doc comments when include_code=false"
    );
    let doc = add_fn["doc"].as_str().unwrap();
    assert!(!doc.is_empty(), "Doc comment should not be empty");
    assert!(doc.contains("Adds"), "Doc should contain description");

    // But should NOT have code
    assert!(
        add_fn["code"].is_null() || !add_fn.as_object().unwrap().contains_key("code"),
        "Should not have code when include_code=false"
    );
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_false_javascript() {
    // Given: JavaScript fixture with include_code=false
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty(), "Should have JavaScript functions");

    for func in functions {
        // Code should be null or absent
        assert!(
            func["code"].is_null() || !func.as_object().unwrap().contains_key("code"),
            "JavaScript function '{}' should NOT have code when include_code=false",
            func["name"]
        );
        // But signature and line should be present
        assert!(
            func["signature"].is_string(),
            "JavaScript function should have signature"
        );
        assert!(
            func["line"].is_number(),
            "JavaScript function should have line number"
        );
    }
}

#[test]
fn test_parse_file_include_code_false_javascript_classes() {
    // Given: JavaScript fixture with classes and include_code=false
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = shape["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should have JavaScript classes");

    for class_item in classes {
        // Code should be null or absent
        assert!(
            class_item["code"].is_null() || !class_item.as_object().unwrap().contains_key("code"),
            "JavaScript class '{}' should NOT have code when include_code=false",
            class_item["name"]
        );
        // But name and line should be present
        assert!(
            class_item["name"].is_string(),
            "JavaScript class should have name"
        );
        assert!(
            class_item["line"].is_number(),
            "JavaScript class should have line number"
        );
    }
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_false_typescript() {
    // Given: TypeScript fixture with include_code=false
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty(), "Should have TypeScript functions");

    for func in functions {
        // Code should be null or absent
        assert!(
            func["code"].is_null() || !func.as_object().unwrap().contains_key("code"),
            "TypeScript function '{}' should NOT have code when include_code=false",
            func["name"]
        );
        // But signature and line should be present
        assert!(
            func["signature"].is_string(),
            "TypeScript function should have signature"
        );
        assert!(
            func["line"].is_number(),
            "TypeScript function should have line number"
        );
    }
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_false_python() {
    // Given: Python fixture with include_code=false
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    assert!(!functions.is_empty(), "Should have Python functions");

    for func in functions {
        // Code should be null or absent
        assert!(
            func["code"].is_null() || !func.as_object().unwrap().contains_key("code"),
            "Python function '{}' should NOT have code when include_code=false",
            func["name"]
        );
        // But signature and line should be present
        assert!(
            func["signature"].is_string(),
            "Python function should have signature"
        );
        assert!(
            func["line"].is_number(),
            "Python function should have line number"
        );
    }
}

#[test]
fn test_parse_file_include_code_false_python_classes() {
    // Given: Python fixture with classes and include_code=false
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = shape["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should have Python classes");

    for class_item in classes {
        // Code should be null or absent
        assert!(
            class_item["code"].is_null() || !class_item.as_object().unwrap().contains_key("code"),
            "Python class '{}' should NOT have code when include_code=false",
            class_item["name"]
        );
        // But name and line should be present
        assert!(
            class_item["name"].is_string(),
            "Python class should have name"
        );
        assert!(
            class_item["line"].is_number(),
            "Python class should have line number"
        );
    }
}

// ============================================================================
// Token Optimization Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_false_reduces_output_size() {
    // Given: Same file parsed with and without code
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // Parse with code
    let args_with_code = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full"
    });
    let result_with_code = treesitter_mcp::analysis::view_code::execute(&args_with_code).unwrap();
    let text_with_code = common::get_result_text(&result_with_code);

    // Parse without code
    let args_without_code = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });
    let result_without_code =
        treesitter_mcp::analysis::view_code::execute(&args_without_code).unwrap();
    let text_without_code = common::get_result_text(&result_without_code);

    // Then: Output without code should be smaller
    assert!(
        text_without_code.len() < text_with_code.len(),
        "Output without code should be smaller than with code"
    );
}

#[test]
fn test_parse_file_include_code_false_preserves_all_metadata() {
    // Given: Rust fixture with include_code=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: All metadata should be preserved
    assert!(shape["language"].is_string(), "Should have language");
    assert!(shape["path"].is_string(), "Should have path");
    assert!(shape["functions"].is_array(), "Should have functions array");
    assert!(shape["imports"].is_array(), "Should have imports array");

    let functions = shape["functions"].as_array().unwrap();
    for func in functions {
        assert!(func["name"].is_string(), "Should have function name");
        assert!(func["signature"].is_string(), "Should have signature");
        assert!(func["line"].is_number(), "Should have line number");
        assert!(func["end_line"].is_number(), "Should have end_line number");
        // doc is optional, but if present should be string
        if func["doc"].is_string() {
            assert!(
                !func["doc"].as_str().unwrap().is_empty(),
                "Doc should not be empty"
            );
        }
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_parse_file_include_code_false_with_empty_functions() {
    // Given: Rust fixture with include_code=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should still succeed even with no code
    assert!(
        result.is_ok(),
        "Should handle include_code=false gracefully"
    );
}

#[test]
fn test_parse_file_include_code_explicit_false_vs_true() {
    // Given: Same file with explicit include_code values
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // Parse with include_code=false
    let args_false = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures"
    });
    let result_false = treesitter_mcp::analysis::view_code::execute(&args_false).unwrap();
    let text_false = common::get_result_text(&result_false);
    let shape_false: serde_json::Value = serde_json::from_str(&text_false).unwrap();

    // Parse with include_code=true
    let args_true = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full"
    });
    let result_true = treesitter_mcp::analysis::view_code::execute(&args_true).unwrap();
    let text_true = common::get_result_text(&result_true);
    let shape_true: serde_json::Value = serde_json::from_str(&text_true).unwrap();

    // Then: Both should have same function names and signatures
    let funcs_false = shape_false["functions"].as_array().unwrap();
    let funcs_true = shape_true["functions"].as_array().unwrap();

    assert_eq!(
        funcs_false.len(),
        funcs_true.len(),
        "Should have same number of functions"
    );

    for (f_false, f_true) in funcs_false.iter().zip(funcs_true.iter()) {
        assert_eq!(
            f_false["name"], f_true["name"],
            "Function names should match"
        );
        assert_eq!(
            f_false["signature"], f_true["signature"],
            "Function signatures should match"
        );
        assert_eq!(
            f_false["line"], f_true["line"],
            "Function line numbers should match"
        );

        // But code should differ
        let has_code_false =
            f_false["code"].is_string() && !f_false["code"].as_str().unwrap().is_empty();
        let has_code_true =
            f_true["code"].is_string() && !f_true["code"].as_str().unwrap().is_empty();

        assert!(!has_code_false, "include_code=false should not have code");
        assert!(has_code_true, "include_code=true should have code");
    }
}
