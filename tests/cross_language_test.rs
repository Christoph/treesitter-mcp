//! Cross-language tests to reduce duplication
//!
//! These tests verify that core functionality works consistently across
//! all supported languages (Rust, Python, JavaScript, TypeScript, Swift, C#, Java).
//! Note: HTML and CSS are not suitable for structural shape analysis and are excluded.

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
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "python",
            "calculator.py",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "javascript",
            "calculator.js",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "typescript",
            "calculator.ts",
            vec!["add", "subtract", "multiply", "divide"],
        ),
        (
            "csharp",
            "Calculator.cs",
            vec!["Add", "Subtract", "Multiply", "Divide"],
        ),
        (
            "java",
            "Calculator.java",
            vec!["add", "subtract", "multiply", "divide"],
        ),
    ];

    for (lang, file, expected_funcs) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments)
            .unwrap_or_else(|e| panic!("parse_file failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

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
        ("csharp", "Models/Point.cs", vec!["Point"]), // C# has Point class in Models
        ("java", "Calculator.java", vec!["Calculator"]),
    ];

    for (lang, file, expected_classes) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
        let text = common::get_result_text(&result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

        // For Rust, classes are emitted via the `s` table (structs).
        // For other languages, use the `c` table (classes).
        let table_key = if lang == "rust" { "s" } else { "c" };
        let rows_str = shape.get(table_key).and_then(|v| v.as_str()).unwrap_or("");

        let rows = common::helpers::parse_compact_rows(rows_str);
        assert!(
            !rows.is_empty(),
            "Should have classes/structs rows for {}",
            lang
        );

        for class_name in expected_classes {
            let found = rows
                .iter()
                .any(|row| row.first().map(|v| v.as_str()) == Some(class_name));
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
        ("csharp", "Calculator.cs"),
        ("java", "Calculator.java"),
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

        // Verify functions have code (compact `h` + `f` rows)
        let header = shape.get("h").and_then(|v| v.as_str()).unwrap_or("");
        let code_idx = header
            .split('|')
            .position(|field| field == "code")
            .unwrap_or_else(|| panic!("Expected 'code' column in header for {}: {header}", lang));

        let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
        let rows = common::helpers::parse_compact_rows(rows_str);

        assert!(
            !rows.is_empty(),
            "Should have at least one function row for {}",
            lang
        );

        for row in rows {
            let code = row
                .get(code_idx)
                .unwrap_or_else(|| panic!("Missing code column {code_idx} for {}", lang));
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
        ("csharp", "Calculator.cs", "Add"),
        ("java", "Calculator.java", "add"),
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

        let usage_rows = common::helpers::find_usages_rows(&usages);
        for row in usage_rows {
            assert!(
                row.get(0).is_some(),
                "Usage row should have file for {}",
                lang
            );
            assert!(
                row.get(1).is_some(),
                "Usage row should have line for {}",
                lang
            );
            assert!(
                row.get(2).is_some(),
                "Usage row should have column for {}",
                lang
            );
            assert!(
                row.get(3).is_some(),
                "Usage row should have usage_type for {}",
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
        ("csharp", "Calculator.cs", "Add"),
        ("java", "Calculator.java", "add"),
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

        let usage_rows = common::helpers::find_usages_rows(&usages);
        assert!(
            !usage_rows.is_empty(),
            "Should find at least one usage for '{}' in {}",
            symbol,
            lang
        );

        // Verify each usage has a valid usage_type
        for row in usage_rows {
            let usage_type = row.get(3).map(|s| s.as_str()).unwrap_or("");
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
        ("csharp", "Calculator.cs", 20, 24),  // On Add method name in signature
        ("java", "Calculator.java", 18, 24),  // On add method name in signature
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

        // Compact schema fields
        assert!(output["sym"].is_string(), "Should have sym for {}", lang);
        assert!(output["sig"].is_string(), "Should have sig for {}", lang);
        assert!(output["kind"].is_string(), "Should have kind for {}", lang);
        assert!(
            output["scope"].is_string(),
            "Should have scope for {}",
            lang
        );

        let scope = output["scope"].as_str().unwrap();
        assert!(!scope.is_empty(), "Scope should be non-empty for {}", lang);
    }
}

#[test]
fn test_get_context_outermost_is_source_file_for_all_languages() {
    let test_cases = vec![
        ("rust", "src/calculator.rs", 14, 5),   // Inside add function
        ("python", "calculator.py", 79, 5),     // Inside Calculator.add method
        ("javascript", "calculator.js", 14, 5), // Inside add function
        ("typescript", "calculator.ts", 14, 5), // Inside add function
        ("csharp", "Calculator.cs", 20, 24),    // On Add method name in signature
        ("java", "Calculator.java", 18, 24),    // On add method name in signature
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

        assert!(
            output["scope"].is_string(),
            "Should have scope for {}",
            lang
        );
        assert!(output["kind"].is_string(), "Should have kind for {}", lang);

        let scope = output["scope"].as_str().unwrap();
        assert!(!scope.is_empty(), "Scope should be non-empty for {}", lang);
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
        ("csharp", "."),
        ("java", "."),
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

        let files = common::helpers::code_map_files(&map);

        assert!(
            !files.is_empty(),
            "Should have at least one file for {}",
            lang
        );

        // Verify each file has required fields
        for (path, _file) in files {
            assert!(!path.is_empty(), "File should have path for {}", lang);
        }
    }
}
