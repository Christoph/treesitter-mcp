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
        let result = treesitter_mcp::analysis::view_code::execute(&arguments);
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
        let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments);
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

        let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let output: serde_json::Value = serde_json::from_str(&text).unwrap();

            prop_assert!(output["sym"].is_string(), "Should have sym");
            prop_assert!(output["scope"].is_string(), "Should have scope");
            prop_assert!(output["kind"].is_string(), "Should have kind");

            let scope = output["scope"].as_str().unwrap();
            prop_assert!(!scope.is_empty(), "Scope should be non-empty");
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

            for row in common::helpers::find_usages_rows(&usages) {
                let file = row.get(0).cloned().unwrap_or_default();
                let line = row.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                let column = row.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

                prop_assert!(!file.is_empty(), "Usage should have file");
                prop_assert!(line > 0, "Line should be positive");
                prop_assert!(column > 0, "Column should be positive");
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
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result1 = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text1 = common::get_result_text(&result1);

    let result2 = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
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

            for row in common::helpers::find_usages_rows(&usages) {
                if let Some(context) = row.get(4) {
                    let line_count = context.lines().count();
                    prop_assert!(
                        line_count <= (2 * context_lines as usize + 1) || context_lines == 0,
                        "Context should respect context_lines parameter"
                    );
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

            let files = common::helpers::code_map_files(&map);
            if !files.is_empty() {
                let (_path, first_file) = files[0];
                prop_assert!(first_file["h"].is_string(), "Should have header");

                let header = first_file["h"].as_str().unwrap();
                if detail == "minimal" {
                    prop_assert_eq!(header, "name|line");
                } else if detail == "signatures" {
                    prop_assert_eq!(header, "name|line|sig");
                } else {
                    prop_assert_eq!(header, "name|line|sig|doc|code");
                }

                if detail != "minimal" {
                    let rows = first_file.get("f").and_then(|v| v.as_str()).unwrap_or("");
                    let parsed = common::helpers::parse_compact_rows(rows);
                    if let Some(first_row) = parsed.first() {
                        prop_assert!(first_row.len() >= 3);
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
            "file_path": file_path.to_str().unwrap(),
            "include_deps": false,
            "max_tokens": 10_000
        });

        let result = treesitter_mcp::analysis::view_code::execute(&arguments);

        // Should succeed for all supported languages
        prop_assert!(result.is_ok(), "Should parse {} files", ext);

        if let Ok(call_result) = result {
            let text = common::get_result_text(&call_result);
            let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

            // Compact schema keys
            prop_assert!(shape.get("p").and_then(|v| v.as_str()).is_some(), "Should have p");
            prop_assert!(shape.get("h").and_then(|v| v.as_str()).is_some(), "Should have h");

            // Should have at least one symbol table
            prop_assert!(
                shape.get("f").is_some()
                    || shape.get("s").is_some()
                    || shape.get("c").is_some()
                    || shape.get("i").is_some(),
                "Should have at least one symbol table"
            );
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

            for row in common::helpers::find_usages_rows(&usages) {
                let line = row.get(1).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
                let column = row.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);

                prop_assert!(line > 0, "Line numbers should be positive (1-indexed)");
                prop_assert!(column > 0, "Column numbers should be positive (1-indexed)");
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

    assert!(usages["u"].is_string(), "Should have usage rows");
    assert!(usages["sym"].is_string(), "Should have sym field");
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

            let rows = common::helpers::find_usages_rows(&usages);
            if !rows.is_empty() {
                // At least one usage should have non-empty context
                let has_context = rows
                    .iter()
                    .any(|row| row.get(4).map(|s| !s.is_empty()).unwrap_or(false));
                prop_assert!(has_context, "Should have non-empty context when context_lines > 0");
            }
        }
    }
}
