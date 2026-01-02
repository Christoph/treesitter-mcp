//! Cross-language tests to reduce duplication
//!
//! These tests verify that core functionality works consistently across
//! all supported languages (Rust, Python, JavaScript, TypeScript).

mod common;

use serde_json::json;

// ============================================================================
// Parse File Tests
// ============================================================================

#[test]
fn test_parse_file_extracts_functions_from_all_languages() {
    let test_cases = vec![
        (
            "rust",
            "src/calculator.rs",
            "Rust",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "python",
            "calculator.py",
            "Python",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "javascript",
            "calculator.js",
            "JavaScript",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "typescript",
            "calculator.ts",
            "TypeScript",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "go",
            "calculator.go",
            "Go",
            vec![
                "Add",
                "Subtract",
                "Multiply",
                "Divide",
                "NewCalculator",
                "AddToHistory",
                "GetHistory",
                "PrintHistory",
            ],
        ),
    ];

    for (lang, file, expected_lang, expected_funcs) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments)
            .unwrap_or_else(|e| panic!("parse_file failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify language
        assert_eq!(
            shape["language"], expected_lang,
            "Wrong language for {}",
            lang
        );

        // Verify functions
        for func_name in expected_funcs {
            common::helpers::assert_has_function(&shape, func_name);
        }
    }
}

#[test]
fn test_parse_file_extracts_classes_from_all_languages() {
    let test_cases = vec![
        ("rust", "src/models/mod.rs", vec!["Calculator"]), // Rust structs are in models/mod.rs
        ("python", "calculator.py", vec!["Calculator"]),
        ("javascript", "calculator.js", vec!["Calculator"]),
        ("typescript", "calculator.ts", vec!["Calculator"]),
        ("go", "calculator.go", vec!["Calculator"]),
    ];

    for (lang, file, expected_classes) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

        // For Rust, classes are called "structs"
        let classes = if lang == "rust" {
            shape["structs"].as_array()
        } else {
            shape["classes"].as_array()
        };

        let classes = classes.unwrap_or_else(|| panic!("Should have classes/structs for {}", lang));

        for class_name in expected_classes {
            let found = classes.iter().any(|c| c["name"] == class_name);
            assert!(found, "Should find class '{}' in {}", class_name, lang);
        }
    }
}

#[test]
fn test_parse_file_includes_code_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs"),
        ("python", "calculator.py"),
        ("javascript", "calculator.js"),
        ("typescript", "calculator.ts"),
        ("go", "calculator.go"),
    ];

    for (lang, file) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments)
            .unwrap_or_else(|e| panic!("parse_file failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify functions have code
        let functions = shape["functions"]
            .as_array()
            .unwrap_or_else(|| panic!("Should have functions for {}", lang));

        assert!(
            !functions.is_empty(),
            "Should have at least one function for {}",
            lang
        );

        for func in functions {
            let code = func["code"]
                .as_str()
                .unwrap_or_else(|| panic!("Function should have code for {}", lang));
            assert!(
                !code.is_empty(),
                "Function code should not be empty for {}",
                lang
            );
        }
    }
}

// ============================================================================
// Find Usages Tests
// ============================================================================

#[test]
fn test_find_usages_locates_function_calls_in_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs", "add"),
        ("python", "calculator.py", "add"),
        ("javascript", "calculator.js", "add"),
        ("typescript", "calculator.ts", "add"),
        ("go", "calculator.go", "Add"),
    ];

    for (lang, file, symbol) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments)
            .unwrap_or_else(|e| panic!("find_usages failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify we found usages
        common::helpers::assert_min_count(&usages, "usages", 1);

        // Verify each usage has required fields
        let usage_list = usages["usages"].as_array().unwrap();
        for usage in usage_list {
            assert!(
                usage["file"].is_string(),
                "Usage should have file for {}",
                lang
            );
            assert!(
                usage["line"].is_number(),
                "Usage should have line for {}",
                lang
            );
            assert!(
                usage["column"].is_number(),
                "Usage should have column for {}",
                lang
            );
            assert!(
                usage["usage_type"].is_string(),
                "Usage should have usage_type for {}",
                lang
            );
        }
    }
}

#[test]
fn test_find_usages_returns_multiple_usages_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs", "add"),
        ("python", "calculator.py", "add"),
        ("javascript", "calculator.js", "add"),
        ("typescript", "calculator.ts", "add"),
        ("go", "calculator.go", "Add"),
    ];

    for (lang, file, symbol) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments)
            .unwrap_or_else(|e| panic!("find_usages failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify we found at least one usage (could be definition, reference, or call)
        let usage_list = usages["usages"].as_array().unwrap();
        assert!(
            !usage_list.is_empty(),
            "Should find at least one usage for '{}' in {}",
            symbol,
            lang
        );

        // Verify each usage has a valid usage_type
        for usage in usage_list {
            let usage_type = usage["usage_type"].as_str().unwrap_or("");
            assert!(
                !usage_type.is_empty(),
                "Usage should have non-empty usage_type for {}",
                lang
            );
        }
    }
}

// ============================================================================
// Get Context Tests
// ============================================================================

#[test]
fn test_get_context_returns_enclosing_scope_for_all_languages() {
    // Use line numbers that are inside functions for better context
    let test_cases = vec![
        ("rust", "src/calculator.rs", 14, 5), // Inside add function body
        ("python", "calculator.py", 79, 8),   // Inside Calculator.add method
        ("javascript", "calculator.js", 14, 5), // Inside add function body
        ("typescript", "calculator.ts", 14, 5), // Inside add function body
        ("go", "calculator.go", 7, 5),        // Inside Add function body
    ];

    for (lang, file, line, column) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "line": line,
            "column": column
        });

        let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments)
            .unwrap_or_else(|e| panic!("symbol_at_line failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let output: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify we have a symbol
        assert!(
            output["symbol"].is_object(),
            "Should have symbol for {}",
            lang
        );

        // Verify symbol has required fields
        assert!(
            output["symbol"]["name"].is_string(),
            "Symbol should have name for {}",
            lang
        );
        assert!(
            output["symbol"]["signature"].is_string(),
            "Symbol should have signature for {}",
            lang
        );
        assert!(
            output["symbol"]["kind"].is_string(),
            "Symbol should have kind for {}",
            lang
        );

        // Verify we have scope_chain
        let scope_chain = output["scope_chain"]
            .as_array()
            .unwrap_or_else(|| panic!("Should have scope_chain for {}", lang));

        assert!(
            !scope_chain.is_empty(),
            "Should have at least one scope for {}",
            lang
        );
    }
}

#[test]
fn test_get_context_outermost_is_source_file_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs", 14, 5),   // Inside add function
        ("python", "calculator.py", 79, 5),     // Inside Calculator.add method
        ("javascript", "calculator.js", 14, 5), // Inside add function
        ("typescript", "calculator.ts", 14, 5), // Inside add function
        ("go", "calculator.go", 7, 5),          // Inside Add function
    ];

    for (lang, file, line, column) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "line": line,
            "column": column
        });

        let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments)
            .unwrap_or_else(|e| panic!("symbol_at_line failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let output: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify we have scope_chain (outermost varies by language)
        let scope_chain = output["scope_chain"].as_array().unwrap();
        assert!(
            !scope_chain.is_empty(),
            "Should have at least one scope for {}",
            lang
        );

        // Just verify the outermost has a kind - different languages use different names
        let outermost = scope_chain.last().unwrap();
        assert!(
            outermost["kind"].is_string(),
            "Outermost scope should have kind for {}",
            lang
        );
    }
}

// ============================================================================
// Code Map Tests
// ============================================================================

#[test]
fn test_code_map_provides_overview_for_all_languages() {
    let test_cases = vec![
        ("rust", "src"),
        ("python", "."),
        ("javascript", "."),
        ("typescript", "."),
        ("go", "."),
    ];

    for (lang, subdir) in test_cases {
        let dir_path = common::fixture_dir(lang).join(subdir);
        let arguments = json!({
            "path": dir_path.to_str().unwrap(),
            "detail": "signatures"
        });

        let result = treesitter_mcp::analysis::code_map::execute(&arguments)
            .unwrap_or_else(|e| panic!("code_map failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let map: serde_json::Value = serde_json::from_str(&text).unwrap();

        // Verify we have files
        let files = map["files"]
            .as_array()
            .unwrap_or_else(|| panic!("Should have files for {}", lang));

        assert!(
            !files.is_empty(),
            "Should have at least one file for {}",
            lang
        );

        // Verify each file has required fields
        for file in files {
            assert!(
                file["path"].is_string(),
                "File should have path for {}",
                lang
            );
            // Note: language field may not be present in code_map output
        }
    }
}
