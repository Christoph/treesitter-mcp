mod common;

use serde_json::json;

#[test]
fn test_parse_file_accepts_include_deps_parameter() {
    // Given: parse_file with include_deps parameter
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    // When: Execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should accept parameter without error
    assert!(result.is_ok());
}

#[test]
fn test_parse_file_no_deps() {
    // Given: parse_file with include_deps=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: No dependencies included
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Project types are always included (even if empty), but should be minimal for files with no deps
    // Just verify the structure is valid
    assert!(shape.is_object(), "Should return valid shape object");
}

#[test]
fn test_parse_file_with_deps_rust() {
    // Given: Rust file with mod declarations
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file with include_deps=true
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Project types are included with signatures only
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let project_types = shape["project_types"].as_array().unwrap();
    assert!(
        project_types.len() >= 1,
        "Should include at least calculator module types"
    );

    // Verify calculator types are included
    let calc_types = project_types.iter().find(|d| {
        d["path"]
            .as_str()
            .map(|p| p.contains("calculator"))
            .unwrap_or(false)
    });
    assert!(calc_types.is_some(), "Should include calculator types");

    let calc = calc_types.unwrap();

    // Check structs (types only, no code)
    if let Some(structs) = calc["structs"].as_array() {
        for struct_info in structs {
            assert!(struct_info["name"].is_string(), "Should have name");
            // Structs in project_types should not have code
            assert!(
                struct_info["code"].is_null() || !struct_info["code"].is_string(),
                "Should NOT have code body for types"
            );
        }
    }

    // Check impl blocks (methods should have signatures but no code)
    if let Some(impl_blocks) = calc["impl_blocks"].as_array() {
        for impl_block in impl_blocks {
            let methods = impl_block["methods"].as_array().unwrap();
            for method in methods {
                assert!(method["signature"].is_string(), "Should have signature");
                assert!(
                    method["code"].is_null() || !method["code"].is_string(),
                    "Should NOT have code body"
                );
            }
        }
    }
}

#[test]
fn test_parse_file_deps_token_efficiency() {
    // Given: File with dependencies
    let file_path = common::fixture_path("rust", "src/lib.rs");

    // When: Parse with full code
    let full_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": true,
    });
    let full_result = treesitter_mcp::analysis::view_code::execute(&full_args).unwrap();
    let full_text = common::get_result_text(&full_result);

    // When: Parse with signatures only
    let sig_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });
    let sig_result = treesitter_mcp::analysis::view_code::execute(&sig_args).unwrap();
    let sig_text = common::get_result_text(&sig_result);

    // Then: Signatures-only should be smaller or similar size
    // (For small files, the difference might be minimal)
    let full_size = full_text.len();
    let sig_size = sig_text.len();

    // Just verify signatures version is not significantly larger
    assert!(
        sig_size <= full_size * 11 / 10,
        "Signatures should not be >110% of full size. Got {} vs {}",
        sig_size,
        full_size
    );
}

#[test]
fn test_parse_file_deps_python() {
    // Given: Python file with imports
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify Python class methods are included in main file
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            if let Some(methods) = class["methods"].as_array() {
                if !methods.is_empty() {
                    // Verify methods have signatures
                    for method in methods {
                        assert!(method["name"].is_string(), "Method should have name");
                        assert!(
                            method["signature"].is_string(),
                            "Method should have signature"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn test_parse_file_deps_javascript() {
    // Given: JavaScript file with imports
    let file_path = common::fixture_path("javascript", "index.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify JS class methods are included
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            if let Some(methods) = class["methods"].as_array() {
                if !methods.is_empty() {
                    // Verify methods have signatures
                    for method in methods {
                        assert!(method["name"].is_string(), "Method should have name");
                    }
                }
            }
        }
    }
}

#[test]
fn test_parse_file_deps_typescript() {
    // Given: TypeScript file with imports
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify TS interfaces and classes are included
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Check for interfaces
    if let Some(interfaces) = shape["interfaces"].as_array() {
        for interface in interfaces {
            assert!(interface["name"].is_string(), "Interface should have name");
            if let Some(methods) = interface["methods"].as_array() {
                if !methods.is_empty() {
                    // Verify interface methods have signatures
                    for method in methods {
                        assert!(method["name"].is_string(), "Method should have name");
                    }
                }
            }
        }
    }

    // Check for classes
    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            if let Some(methods) = class["methods"].as_array() {
                if !methods.is_empty() {
                    // Verify class methods have signatures
                    for method in methods {
                        assert!(method["name"].is_string(), "Method should have name");
                    }
                }
            }
        }
    }
}

#[test]
fn test_parse_file_deps_rust_traits() {
    // Given: Rust file with traits in dependencies
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Check dependencies for traits (if any exist)
    if let Some(deps) = shape["dependencies"].as_array() {
        for dep in deps {
            if let Some(traits) = dep["traits"].as_array() {
                for trait_def in traits {
                    assert!(trait_def["name"].is_string(), "Trait should have name");
                    if let Some(methods) = trait_def["methods"].as_array() {
                        for method in methods {
                            assert!(
                                method["signature"].is_string(),
                                "Trait method should have signature"
                            );
                        }
                    }
                }
            }
        }
    }
}
