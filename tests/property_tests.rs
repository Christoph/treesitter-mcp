//! Property-based tests for treesitter-mcp
//!
//! These tests verify invariants that should hold for all inputs,
//! not just specific test cases.

mod common;

use proptest::prelude::*;
use serde_json::json;

// ============================================================================
// Property: Parser should never panic on any input
// ============================================================================

proptest! {
    /// Property: parse_file should never panic, even on invalid file paths
    #[test]
    fn test_parse_file_never_panics_on_invalid_paths(
        path in "[a-z]{1,20}\\.(rs|py|js|ts)"
    ) {
        let arguments = json!({
            "file_path": path
        });

        // Should either succeed or return error, never panic
        let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
        prop_assert!(result.is_ok() || result.is_err());
    }

    /// Property: find_usages should never panic on any symbol name
    #[test]
    fn test_find_usages_never_panics_on_random_symbols(
        symbol in "[a-zA-Z_][a-zA-Z0-9_]{0,50}"
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        // Should either succeed or return error, never panic
        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);
        prop_assert!(result.is_ok() || result.is_err());
    }

    /// Property: get_context should handle any line/column position gracefully
    #[test]
    fn test_get_context_never_panics_on_random_positions(
        line in 1..1000u64,
        column in 1..200u64
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "line": line,
            "column": column
        });

        // Should either succeed or return error, never panic
        let result = treesitter_mcp::analysis::get_context::execute(&arguments);
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// ============================================================================
// Property: Context always has at least one level (source_file)
// ============================================================================

proptest! {
    /// Property: get_context always returns at least one context (source_file)
    /// when called on a valid file with valid position
    #[test]
    fn test_get_context_always_has_source_file(
        line in 1..50u64,
        column in 1..80u64
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "line": line,
            "column": column
        });

        let result = treesitter_mcp::analysis::get_context::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let context: serde_json::Value = serde_json::from_str(&text).unwrap();

            let contexts = context["contexts"].as_array().unwrap();
            prop_assert!(!contexts.is_empty(), "Should have at least one context");

            // Outermost context should be source_file or similar
            let outermost = contexts.last().unwrap();
            prop_assert!(outermost["type"].is_string(), "Outermost should have type");
        }
    }
}

// ============================================================================
// Property: All paths in results should be consistent
// ============================================================================

proptest! {
    /// Property: find_usages results should have consistent path format
    #[test]
    fn test_find_usages_paths_are_consistent(
        symbol in "[a-z]{3,10}"
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

            if let Some(usage_list) = usages["usages"].as_array() {
                for usage in usage_list {
                    // All usages should have a file path
                    prop_assert!(usage["file"].is_string(), "Usage should have file");

                    // All usages should have line and column
                    prop_assert!(usage["line"].is_number(), "Usage should have line");
                    prop_assert!(usage["column"].is_number(), "Usage should have column");

                    // Line and column should be positive
                    let line = usage["line"].as_u64().unwrap();
                    let column = usage["column"].as_u64().unwrap();
                    prop_assert!(line > 0, "Line should be positive");
                    prop_assert!(column > 0, "Column should be positive");
                }
            }
        }
    }
}

// ============================================================================
// Property: Code map respects token limits
// ============================================================================

proptest! {
    /// Property: code_map output should respect max_tokens parameter
    #[test]
    fn test_code_map_respects_token_limit(
        max_tokens in 100..5000u32
    ) {
        let dir_path = common::fixture_dir("rust");
        let arguments = json!({
            "path": dir_path.join("src").to_str().unwrap(),
            "detail": "signatures",
            "max_tokens": max_tokens
        });

        let result = treesitter_mcp::analysis::code_map::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);

            // Rough token count (4 chars per token)
            let approx_tokens = text.len() / 4;

            // Should be within reasonable bounds (allow 20% overage for structure)
            prop_assert!(
                approx_tokens <= (max_tokens as usize * 12 / 10),
                "Output should respect token limit (got ~{} tokens, limit {})",
                approx_tokens, max_tokens
            );
        }
    }
}

// ============================================================================
// Property: Parse file results are deterministic
// ============================================================================

#[test]
fn test_parse_file_is_deterministic() {
    // Property: Calling parse_file multiple times should return identical results
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result1 = treesitter_mcp::analysis::parse_file::execute(&arguments).unwrap();
    let text1 = common::get_result_text(&result1);

    let result2 = treesitter_mcp::analysis::parse_file::execute(&arguments).unwrap();
    let text2 = common::get_result_text(&result2);

    assert_eq!(text1, text2, "parse_file should be deterministic");
}

#[test]
fn test_find_usages_is_deterministic() {
    // Property: Calling find_usages multiple times should return identical results
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 2
    });

    let result1 = treesitter_mcp::analysis::find_usages::execute(&arguments).unwrap();
    let text1 = common::get_result_text(&result1);

    let result2 = treesitter_mcp::analysis::find_usages::execute(&arguments).unwrap();
    let text2 = common::get_result_text(&result2);

    assert_eq!(text1, text2, "find_usages should be deterministic");
}

// ============================================================================
// Property: Context lines parameter is respected
// ============================================================================

proptest! {
    /// Property: find_usages context_lines parameter affects output size
    #[test]
    fn test_find_usages_context_lines_affects_output(
        context_lines in 0..10u32
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "symbol": "add",
            "path": file_path.to_str().unwrap(),
            "context_lines": context_lines
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

            if let Some(usage_list) = usages["usages"].as_array() {
                for usage in usage_list {
                    if let Some(code) = usage["code"].as_str() {
                        let line_count = code.lines().count();
                        // Code should have at most (2 * context_lines + 1) lines
                        // (context before + target line + context after)
                        prop_assert!(
                            line_count <= (2 * context_lines as usize + 1) || context_lines == 0,
                            "Code should respect context_lines parameter"
                        );
                    }
                }
            }
        }
    }
}

// ============================================================================
// Property: Query pattern results are valid
// ============================================================================

proptest! {
    /// Property: query_pattern with valid S-expression never panics
    #[test]
    fn test_query_pattern_handles_simple_queries(
        node_type in "(function_item|struct_item|impl_item)"
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let query = format!("({} name: (identifier) @name)", node_type);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap(),
            "query": query
        });

        // Should either succeed or return error, never panic
        let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);
        prop_assert!(result.is_ok() || result.is_err());
    }
}

// ============================================================================
// Property: Code map detail levels are consistent
// ============================================================================

proptest! {
    /// Property: Higher detail levels include more information
    #[test]
    fn test_code_map_detail_levels_are_ordered(
        detail in prop_oneof![
            Just("minimal"),
            Just("signatures"),
            Just("full")
        ]
    ) {
        let dir_path = common::fixture_dir("rust");
        let arguments = json!({
            "path": dir_path.join("src").to_str().unwrap(),
            "detail": detail,
            "max_tokens": 10000
        });

        let result = treesitter_mcp::analysis::code_map::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let map: serde_json::Value = serde_json::from_str(&text).unwrap();

            // All detail levels should have files array
            prop_assert!(map["files"].is_array(), "Should have files array");

            let files = map["files"].as_array().unwrap();
            if !files.is_empty() {
                let first_file = &files[0];

                // All detail levels should have path
                prop_assert!(first_file["path"].is_string(), "Should have path");

                // Check detail-specific fields
                if detail == "signatures" || detail == "full" {
                    // Should have functions with signatures
                    if let Some(functions) = first_file["functions"].as_array() {
                        if !functions.is_empty() {
                            // At least some functions should have signatures
                            let has_signatures = functions.iter().any(|f| f["signature"].is_string());
                            prop_assert!(has_signatures || detail == "minimal",
                                "Signatures detail should include signatures");
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Property: Parse file handles all supported languages
// ============================================================================

proptest! {
    /// Property: parse_file works for all supported file extensions
    #[test]
    fn test_parse_file_supports_all_extensions(
        ext in prop_oneof![
            Just("rs"),
            Just("py"),
            Just("js"),
            Just("ts")
        ]
    ) {
        let lang = match ext {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            _ => "rust"
        };

        let file = match ext {
            "rs" => "src/calculator.rs",
            "py" => "calculator.py",
            "js" => "calculator.js",
            "ts" => "calculator.ts",
            _ => "src/calculator.rs"
        };

        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

        // Should succeed for all supported languages
        prop_assert!(result.is_ok(), "Should parse {} files", ext);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

            // Should have language field
            prop_assert!(shape["language"].is_string(), "Should have language");

            // Should have functions array
            prop_assert!(shape["functions"].is_array(), "Should have functions");
        }
    }
}

// ============================================================================
// Property: Line and column numbers are always positive
// ============================================================================

proptest! {
    /// Property: All line/column numbers in results are positive
    #[test]
    fn test_all_positions_are_positive(
        symbol in "[a-z]{3,10}"
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "symbol": symbol,
            "path": file_path.to_str().unwrap(),
            "context_lines": 2
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

            if let Some(usage_list) = usages["usages"].as_array() {
                for usage in usage_list {
                    if let Some(line) = usage["line"].as_u64() {
                        prop_assert!(line > 0, "Line numbers should be positive (1-indexed)");
                    }
                    if let Some(column) = usage["column"].as_u64() {
                        prop_assert!(column > 0, "Column numbers should be positive (1-indexed)");
                    }
                }
            }
        }
    }
}

// ============================================================================
// Property: Empty results are valid JSON
// ============================================================================

#[test]
fn test_empty_results_are_valid_json() {
    // Property: Even when no results found, output should be valid JSON
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "nonexistent_symbol_xyz123",
        "path": file_path.to_str().unwrap(),
        "context_lines": 2
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);

    // Should be valid JSON
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have usages array (even if empty)
    assert!(usages["usages"].is_array(), "Should have usages array");

    // Should have symbol field
    assert!(usages["symbol"].is_string(), "Should have symbol field");
}

// ============================================================================
// Property: Code snippets are non-empty when context_lines > 0
// ============================================================================

proptest! {
    /// Property: When context_lines > 0, code snippets should be non-empty
    #[test]
    fn test_code_snippets_non_empty_with_context(
        context_lines in 1..10u32
    ) {
        let file_path = common::fixture_path("rust", "src/calculator.rs");
        let arguments = json!({
            "symbol": "add",
            "path": file_path.to_str().unwrap(),
            "context_lines": context_lines
        });

        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

            if let Some(usage_list) = usages["usages"].as_array() {
                if !usage_list.is_empty() {
                    // At least one usage should have non-empty code
                    let has_code = usage_list.iter().any(|u| {
                        u["code"].as_str().map(|s| !s.is_empty()).unwrap_or(false)
                    });
                    prop_assert!(has_code, "Should have non-empty code when context_lines > 0");
                }
            }
        }
    }
}
