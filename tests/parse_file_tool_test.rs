use serde_json::json;

mod common;

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_parse_file_rust_functions() {
    // Given: Rust fixture with functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with function names, signatures, line numbers
    assert!(result.is_ok(), "parse_file should succeed");
    let call_result = result.unwrap();
    assert!(
        !call_result.is_error.unwrap_or(false),
        "Should not be an error"
    );

    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have path and language
    assert_eq!(shape["language"], "Rust");
    assert!(shape["path"].as_str().unwrap().contains("calculator.rs"));

    // Should have functions array
    assert!(shape["functions"].is_array());
    let functions = shape["functions"].as_array().unwrap();
    assert!(functions.len() >= 5); // add, subtract, multiply, divide, apply_operation, create_calculator

    // Check for specific function
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    assert_eq!(add_fn["name"], "add");
    assert!(add_fn["signature"].as_str().unwrap().contains("pub fn add"));
    assert!(add_fn["signature"].as_str().unwrap().contains("i32"));
    assert!(add_fn["line"].as_u64().unwrap() > 0);
    assert!(add_fn["end_line"].as_u64().unwrap() > add_fn["line"].as_u64().unwrap());

    // Verify the actual code is included
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("a + b"),
            "Code should contain the actual implementation"
        );
        assert!(
            code.contains("pub fn add"),
            "Code should contain the function signature"
        );
    }
}

#[test]
fn test_parse_file_rust_structs() {
    // Given: Rust fixture with structs
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with struct names, line numbers
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(shape["structs"].is_array());
    let structs = shape["structs"].as_array().unwrap();
    assert!(structs.len() >= 2); // Calculator, Point

    // Check for Calculator struct
    let calc_struct = structs.iter().find(|s| s["name"] == "Calculator").unwrap();
    assert_eq!(calc_struct["name"], "Calculator");
    assert!(calc_struct["line"].as_u64().unwrap() > 0);

    // Verify code is present for struct
    if calc_struct["code"].is_string() {
        let code = calc_struct["code"].as_str().unwrap();
        assert!(
            code.contains("Calculator"),
            "Code should contain struct name"
        );
        assert!(
            code.contains("pub value: i32"),
            "Code should contain struct fields"
        );
    }
}

#[test]
fn test_parse_file_rust_docs() {
    // Given: Rust fixture with doc comments
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with doc strings extracted
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    // Should have doc comment
    assert!(add_fn["doc"].is_string());
    let doc = add_fn["doc"].as_str().unwrap();
    assert!(
        doc.contains("Adds two numbers"),
        "Doc should contain description"
    );

    // Verify code is present and correct
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(code.contains("a + b"), "Code should contain implementation");
    }
}

#[test]
fn test_parse_file_rust_imports() {
    // Given: Rust fixture with imports
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with imports
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(shape["imports"].is_array());
    let imports = shape["imports"].as_array().unwrap();
    assert!(imports.len() > 0);

    // Should have std::fmt import
    let has_fmt_import = imports
        .iter()
        .any(|i| i["text"].as_str().unwrap().contains("std::fmt"));
    assert!(has_fmt_import);
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_parse_file_python_functions() {
    // Given: Python fixture with functions
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with function names, signatures
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(shape["language"], "Python");
    assert!(shape["functions"].is_array());

    let functions = shape["functions"].as_array().unwrap();
    assert!(functions.len() >= 4); // add, subtract, multiply, divide, apply_operation

    // Check for add function
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    assert_eq!(add_fn["name"], "add");
    assert!(add_fn["signature"].as_str().unwrap().contains("def add"));

    // Verify code is present and correct
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("return a + b"),
            "Code should contain Python implementation"
        );
        assert!(
            code.contains("def add"),
            "Code should contain function definition"
        );
    }
}

#[test]
fn test_parse_file_python_classes() {
    // Given: Python fixture with classes
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with class names, methods
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(shape["classes"].is_array());
    let classes = shape["classes"].as_array().unwrap();
    assert!(classes.len() >= 2); // Calculator, Point

    // Check for Calculator class
    let calc_class = classes.iter().find(|c| c["name"] == "Calculator").unwrap();
    assert_eq!(calc_class["name"], "Calculator");
    assert!(calc_class["line"].as_u64().unwrap() > 0);

    // Verify code is present for class
    if calc_class["code"].is_string() {
        let code = calc_class["code"].as_str().unwrap();
        assert!(
            code.contains("class Calculator"),
            "Code should contain class definition"
        );
        assert!(
            code.contains("def __init__"),
            "Code should contain constructor"
        );
    }
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_parse_file_javascript_functions() {
    // Given: JavaScript fixture with functions
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with function names
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(shape["language"], "JavaScript");
    assert!(shape["functions"].is_array());

    let functions = shape["functions"].as_array().unwrap();
    assert!(functions.len() >= 4); // add, subtract, multiply, divide, etc.

    // Verify at least one function has code
    let add_fn = functions.iter().find(|f| f["name"] == "add");
    if let Some(add_fn) = add_fn {
        if add_fn["code"].is_string() {
            let code = add_fn["code"].as_str().unwrap();
            assert!(
                code.contains("return") || code.contains("a + b"),
                "Code should contain implementation"
            );
        }
    }
}

#[test]
fn test_parse_file_javascript_classes() {
    // Given: JavaScript fixture with ES6 classes
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with class names, methods
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(shape["classes"].is_array());
    let classes = shape["classes"].as_array().unwrap();
    assert!(classes.len() >= 2); // Calculator, Point
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_parse_file_typescript_with_types() {
    // Given: TypeScript fixture with interfaces and typed functions
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with types, interfaces, typed functions
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(shape["language"], "TypeScript");

    // Should have functions with type signatures
    let functions = shape["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    assert!(add_fn["signature"].as_str().unwrap().contains("number"));

    // Verify code is present and correct
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("return") || code.contains("a + b"),
            "Code should contain TypeScript implementation"
        );
    }
}

#[test]
fn test_parse_file_typescript_interfaces() {
    // Given: TypeScript fixture with interfaces
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns JSON with interfaces
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // TypeScript interfaces might be in classes or a separate field
    // depending on implementation
    assert!(shape["classes"].is_array() || shape["interfaces"].is_array());
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_parse_file_nonexistent_file() {
    // Given: Path to non-existent file
    let arguments = json!({
        "file_path": "/nonexistent/file.rs"
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns error
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_parse_file_unsupported_extension() {
    // Given: File with unsupported extension
    use std::fs;
    use tempfile::TempDir;
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "plain text").unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns error
    assert!(
        result.is_err(),
        "Should return error for unsupported extension"
    );
}

// ============================================================================
// Code Content Verification Tests
// ============================================================================

#[test]
fn test_parse_file_rust_code_matches_fixture() {
    // Given: Rust fixture with known content
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Code in result matches actual fixture content
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();

    // Verify add function code
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("pub fn add(a: i32, b: i32) -> i32"),
            "Should have exact signature"
        );
        assert!(code.contains("a + b"), "Should have exact implementation");
    }

    // Verify subtract function code
    let sub_fn = functions.iter().find(|f| f["name"] == "subtract").unwrap();
    if sub_fn["code"].is_string() {
        let code = sub_fn["code"].as_str().unwrap();
        assert!(
            code.contains("a - b"),
            "Should have subtract implementation"
        );
    }
}

#[test]
fn test_parse_file_python_code_matches_fixture() {
    // Given: Python fixture with known content
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Code in result matches actual fixture content
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let functions = shape["functions"].as_array().unwrap();

    // Verify add function code
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("def add(a, b):"),
            "Should have Python function signature"
        );
        assert!(
            code.contains("return a + b"),
            "Should have Python implementation"
        );
    }

    // Verify doc string is extracted
    if add_fn["doc"].is_string() {
        let doc = add_fn["doc"].as_str().unwrap();
        assert!(
            doc.contains("Adds two numbers together"),
            "Should extract docstring"
        );
    }
}
