mod common;

use serde_json::json;

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate total lines of code across all usages
fn calculate_total_context_lines(usages: &serde_json::Value) -> usize {
    if let Some(usage_list) = usages["usages"].as_array() {
        let mut total = 0;
        for usage in usage_list {
            if let Some(code) = usage["code"].as_str() {
                total += code.lines().count();
            }
        }
        total
    } else {
        0
    }
}

// ============================================================================
// Test 1: max_context_lines caps total context
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_caps_total_context() {
    // Given: Rust project with symbol that has multiple usages
    let project_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "add",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 50
    });

    // When: find_usages is called with max_context_lines
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Total context lines should be capped at ~50
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_lines = calculate_total_context_lines(&usages);

    // Allow 20% buffer for edge cases
    assert!(
        total_lines <= 60,
        "Total context lines ({}) should be capped at ~50 (max 60 with buffer)",
        total_lines
    );
}

// ============================================================================
// Test 2: max_context_lines=0 returns no code snippets
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_zero_returns_no_code() {
    // Given: Rust fixture with max_context_lines=0
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 0
    });

    // When: find_usages is called with max_context_lines=0
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Usages should have no code snippets
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(!usage_list.is_empty(), "Should find at least one usage");

    // All usages should have no code or empty code
    for usage in usage_list {
        let code = usage["code"].as_str().unwrap_or("");
        assert!(
            code.is_empty(),
            "Code should be empty when max_context_lines=0"
        );
    }

    // But file/line/column should still be present
    for usage in usage_list {
        assert!(usage["file"].is_string(), "File should be present");
        assert!(usage["line"].is_number(), "Line should be present");
        assert!(usage["column"].is_number(), "Column should be present");
    }
}

// ============================================================================
// Test 3: max_context_lines respects context_lines parameter
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_respects_context_lines_param() {
    // Given: Rust fixture with both context_lines and max_context_lines
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // Test with context_lines=2
    let arguments_small = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 2,
        "max_context_lines": 100
    });

    let result_small = treesitter_mcp::analysis::find_usages::execute(&arguments_small);
    assert!(result_small.is_ok());
    let call_result_small = result_small.unwrap();
    let text_small = common::get_result_text(&call_result_small);
    let usages_small: serde_json::Value = serde_json::from_str(&text_small).unwrap();

    // Test with context_lines=5
    let arguments_large = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 5,
        "max_context_lines": 100
    });

    let result_large = treesitter_mcp::analysis::find_usages::execute(&arguments_large);
    assert!(result_large.is_ok());
    let call_result_large = result_large.unwrap();
    let text_large = common::get_result_text(&call_result_large);
    let usages_large: serde_json::Value = serde_json::from_str(&text_large).unwrap();

    // With larger context_lines, total should be larger (or equal if only one usage)
    let total_small = calculate_total_context_lines(&usages_small);
    let total_large = calculate_total_context_lines(&usages_large);

    // Either they're equal (single usage) or large is bigger
    assert!(
        total_large >= total_small,
        "Larger context_lines should produce more or equal total lines"
    );
}

// ============================================================================
// Test 4: Without max_context_lines returns all usages (backward compatibility)
// ============================================================================

#[test]
fn test_find_usages_without_max_context_lines_returns_all() {
    // Given: Rust fixture without max_context_lines parameter
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3
    });

    // When: find_usages is called without max_context_lines
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should return all usages (backward compatible)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(!usage_list.is_empty(), "Should find usages");

    // All usages should have code snippets
    for usage in usage_list {
        assert!(
            usage["code"].is_string(),
            "Code should be present without max_context_lines"
        );
    }
}

// ============================================================================
// Test 5: Single usage not truncated
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_with_single_usage() {
    // Given: Rust fixture with a symbol that has only one usage
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "multiply",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 20
    });

    // When: find_usages is called
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Single usage should not be truncated
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Should have at least the definition
    assert!(usage_list.len() >= 1, "Should find at least one usage");

    // The single usage should have its full context
    if usage_list.len() == 1 {
        let usage = &usage_list[0];
        assert!(
            usage["code"].is_string(),
            "Single usage should have code snippet"
        );
        let code = usage["code"].as_str().unwrap();
        assert!(!code.is_empty(), "Single usage code should not be empty");
    }
}

// ============================================================================
// Test 6: Many usages truncated gracefully
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_truncates_gracefully() {
    // Given: Rust project with symbol that has many usages
    let project_path = common::fixture_dir("rust");
    let arguments_unlimited = json!({
        "symbol": "add",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 3
    });

    let arguments_limited = json!({
        "symbol": "add",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 30
    });

    // When: find_usages is called with and without limit
    let result_unlimited = treesitter_mcp::analysis::find_usages::execute(&arguments_unlimited);
    let result_limited = treesitter_mcp::analysis::find_usages::execute(&arguments_limited);

    assert!(result_unlimited.is_ok());
    assert!(result_limited.is_ok());

    let call_result_unlimited = result_unlimited.unwrap();
    let text_unlimited = common::get_result_text(&call_result_unlimited);
    let usages_unlimited: serde_json::Value = serde_json::from_str(&text_unlimited).unwrap();

    let call_result_limited = result_limited.unwrap();
    let text_limited = common::get_result_text(&call_result_limited);
    let usages_limited: serde_json::Value = serde_json::from_str(&text_limited).unwrap();

    let total_unlimited = calculate_total_context_lines(&usages_unlimited);
    let total_limited = calculate_total_context_lines(&usages_limited);

    // Then: Limited should have fewer or equal total lines
    assert!(
        total_limited <= total_unlimited,
        "Limited context ({}) should be <= unlimited ({})",
        total_limited,
        total_unlimited
    );

    // Limited should be capped at ~30
    assert!(
        total_limited <= 36, // 20% buffer
        "Limited context ({}) should be capped at ~30",
        total_limited
    );
}

// ============================================================================
// Test 7: max_context_lines works for Python
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_python() {
    // Given: Python fixture with max_context_lines
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 40
    });

    // When: find_usages is called on Python file
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should respect max_context_lines for Python
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_lines = calculate_total_context_lines(&usages);

    // Should be capped at ~40
    assert!(
        total_lines <= 48, // 20% buffer
        "Python total context lines ({}) should be capped at ~40",
        total_lines
    );

    // Verify metadata is present
    let usage_list = usages["usages"].as_array().unwrap();
    for usage in usage_list {
        assert!(usage["file"].is_string(), "File should be present");
        assert!(usage["line"].is_number(), "Line should be present");
        assert!(usage["column"].is_number(), "Column should be present");
    }
}

// ============================================================================
// Test 8: max_context_lines works for JavaScript
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_javascript() {
    // Given: JavaScript fixture with max_context_lines
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 35
    });

    // When: find_usages is called on JavaScript file
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should respect max_context_lines for JavaScript
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_lines = calculate_total_context_lines(&usages);

    // Should be capped at ~35
    assert!(
        total_lines <= 42, // 20% buffer
        "JavaScript total context lines ({}) should be capped at ~35",
        total_lines
    );

    // Verify metadata is present
    let usage_list = usages["usages"].as_array().unwrap();
    for usage in usage_list {
        assert!(usage["file"].is_string(), "File should be present");
        assert!(usage["line"].is_number(), "Line should be present");
        assert!(usage["column"].is_number(), "Column should be present");
    }
}

// ============================================================================
// Test 9: Very small cap (e.g., 10 lines)
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_small_cap() {
    // Given: Rust fixture with very small max_context_lines
    let project_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "add",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 10
    });

    // When: find_usages is called with small cap
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should respect very small cap
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_lines = calculate_total_context_lines(&usages);

    // Should be capped at ~10
    assert!(
        total_lines <= 12, // 20% buffer
        "Total context lines ({}) should be capped at ~10",
        total_lines
    );

    // Should still have at least one usage
    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 1, "Should have at least one usage");
}

// ============================================================================
// Test 10: Metadata always present regardless of max_context_lines
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_preserves_metadata() {
    // Given: Rust fixture with various max_context_lines values
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    let test_cases = vec![
        ("max=0", 0),
        ("max=10", 10),
        ("max=50", 50),
        ("max=100", 100),
    ];

    for (label, max_lines) in test_cases {
        let arguments = json!({
            "symbol": "add",
            "path": file_path.to_str().unwrap(),
            "context_lines": 3,
            "max_context_lines": max_lines
        });

        // When: find_usages is called
        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        // Then: Metadata should always be present
        assert!(result.is_ok(), "Should succeed for {}", label);
        let call_result = result.unwrap();
        let text = common::get_result_text(&call_result);
        let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

        let usage_list = usages["usages"].as_array().unwrap();

        for usage in usage_list {
            assert!(
                usage["file"].is_string(),
                "File should be present for {}",
                label
            );
            assert!(
                usage["line"].is_number(),
                "Line should be present for {}",
                label
            );
            assert!(
                usage["column"].is_number(),
                "Column should be present for {}",
                label
            );

            // Verify values are reasonable
            let line = usage["line"].as_u64().unwrap();
            let column = usage["column"].as_u64().unwrap();
            assert!(line > 0, "Line should be > 0 for {}", label);
            assert!(column > 0, "Column should be > 0 for {}", label);
        }
    }
}

// ============================================================================
// Test 11: max_context_lines with cross-file search
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_cross_file() {
    // Given: Rust project with cross-file symbol
    let project_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 2,
        "max_context_lines": 45
    });

    // When: find_usages is called on directory
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should respect max_context_lines across files
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_lines = calculate_total_context_lines(&usages);

    // Should be capped at ~45
    assert!(
        total_lines <= 54, // 20% buffer
        "Cross-file total context lines ({}) should be capped at ~45",
        total_lines
    );

    // Should have usages from multiple files
    let usage_list = usages["usages"].as_array().unwrap();
    let files: std::collections::HashSet<_> = usage_list
        .iter()
        .map(|u| u["file"].as_str().unwrap())
        .collect();

    // May have multiple files or single file depending on fixture
    assert!(files.len() >= 1, "Should have at least one file");
}

// ============================================================================
// Test 12: max_context_lines with different context_lines values
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_with_varying_context() {
    // Given: Rust fixture with different context_lines values
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    let test_cases = vec![(1, 30), (2, 30), (3, 30), (5, 30)];

    for (context_lines, max_lines) in test_cases {
        let arguments = json!({
            "symbol": "add",
            "path": file_path.to_str().unwrap(),
            "context_lines": context_lines,
            "max_context_lines": max_lines
        });

        // When: find_usages is called
        let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

        // Then: Should respect max_context_lines regardless of context_lines
        assert!(result.is_ok());
        let call_result = result.unwrap();
        let text = common::get_result_text(&call_result);
        let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

        let total_lines = calculate_total_context_lines(&usages);

        // Should be capped at ~30
        assert!(
            total_lines <= 36, // 20% buffer
            "Total context lines ({}) should be capped at ~30 (context_lines={})",
            total_lines,
            context_lines
        );
    }
}

// ============================================================================
// Test 13: max_context_lines with rare symbols
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_rare_symbol() {
    // Given: Rust fixture with rare symbol (few usages)
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "symbol": "divide",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 20
    });

    // When: find_usages is called for rare symbol
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should return all usages (not truncated if under cap)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();

    // Should find at least the definition
    assert!(usage_list.len() >= 1, "Should find at least one usage");

    // All usages should have code (since they're under cap)
    for usage in usage_list {
        assert!(
            usage["code"].is_string(),
            "Rare symbol usages should have code"
        );
    }
}

// ============================================================================
// Test 14: max_context_lines=0 with multiple usages
// ============================================================================

#[test]
 // Feature not yet implemented
fn test_find_usages_max_context_lines_zero_multiple_usages() {
    // Given: Rust project with max_context_lines=0 and multiple usages
    let project_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "add",
        "path": project_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 0
    });

    // When: find_usages is called with max_context_lines=0
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should return all usages but with no code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(!usage_list.is_empty(), "Should find usages");

    // All usages should have no code
    for usage in usage_list {
        let code = usage["code"].as_str().unwrap_or("");
        assert!(
            code.is_empty(),
            "Code should be empty with max_context_lines=0"
        );

        // But metadata should be present
        assert!(usage["file"].is_string(), "File should be present");
        assert!(usage["line"].is_number(), "Line should be present");
        assert!(usage["column"].is_number(), "Column should be present");
    }

    // Total lines should be 0
    let total_lines = calculate_total_context_lines(&usages);
    assert_eq!(
        total_lines, 0,
        "Total lines should be 0 with max_context_lines=0"
    );
}

// ============================================================================
// Test 15: Large max_context_lines doesn't affect results
// ============================================================================

#[test]
fn test_find_usages_max_context_lines_large_cap() {
    // Given: Rust fixture with very large max_context_lines
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    let arguments_no_cap = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3
    });

    let arguments_large_cap = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 10000
    });

    // When: find_usages is called with and without large cap
    let result_no_cap = treesitter_mcp::analysis::find_usages::execute(&arguments_no_cap);
    let result_large_cap = treesitter_mcp::analysis::find_usages::execute(&arguments_large_cap);

    assert!(result_no_cap.is_ok());
    assert!(result_large_cap.is_ok());

    let call_result_no_cap = result_no_cap.unwrap();
    let text_no_cap = common::get_result_text(&call_result_no_cap);
    let usages_no_cap: serde_json::Value = serde_json::from_str(&text_no_cap).unwrap();

    let call_result_large_cap = result_large_cap.unwrap();
    let text_large_cap = common::get_result_text(&call_result_large_cap);
    let usages_large_cap: serde_json::Value = serde_json::from_str(&text_large_cap).unwrap();

    // Then: Results should be identical (large cap doesn't affect results)
    let usage_list_no_cap = usages_no_cap["usages"].as_array().unwrap();
    let usage_list_large_cap = usages_large_cap["usages"].as_array().unwrap();

    assert_eq!(
        usage_list_no_cap.len(),
        usage_list_large_cap.len(),
        "Should have same number of usages"
    );

    let total_no_cap = calculate_total_context_lines(&usages_no_cap);
    let total_large_cap = calculate_total_context_lines(&usages_large_cap);

    assert_eq!(
        total_no_cap, total_large_cap,
        "Should have same total context lines"
    );
}
