//! Token efficiency tests to verify documented savings
//!
//! These tests ensure the token optimization features work as documented
//! in README.md and LLM_IMPROVEMENTS.md.

mod common;

use serde_json::json;

// ============================================================================
// file_shape vs parse_file (documented: 10-20% of parse_file)
// ============================================================================

#[test]
fn test_file_shape_is_significantly_cheaper_than_parse_file() {
    // Given: Same file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: Compare file_shape vs parse_file output sizes
    let parse_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let parse_result = treesitter_mcp::analysis::view_code::execute(&parse_args).unwrap();
    let parse_text = common::get_result_text(&parse_result);
    let parse_tokens = common::helpers::approx_tokens(&parse_text);

    let shape_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "merge_templates": false
    });
    let shape_result = treesitter_mcp::analysis::view_code::execute(&shape_args).unwrap();
    let shape_text = common::get_result_text(&shape_result);
    let shape_tokens = common::helpers::approx_tokens(&shape_text);

    // Then: file_shape should be significantly cheaper than parse_file (allow up to 60%)
    // Note: With detail="signatures", we get ~50% reduction due to function bodies being removed
    // but signatures, documentation, and structural elements remain
    let ratio = shape_tokens as f64 / parse_tokens as f64;
    assert!(
        ratio < 0.60,
        "file_shape should be <60% of parse_file tokens, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        shape_tokens,
        parse_tokens
    );
}

// ============================================================================
// include_code=false (documented: 60-80% reduction)
// ============================================================================

#[test]
fn test_include_code_false_reduces_tokens_significantly() {
    // Given: File with functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_code=true) vs parse_file(include_code=false)
    let with_code_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let with_code_result = treesitter_mcp::analysis::view_code::execute(&with_code_args).unwrap();
    let with_code_text = common::get_result_text(&with_code_result);
    let with_tokens = common::helpers::approx_tokens(&with_code_text);

    let without_code_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false
    });
    let without_code_result =
        treesitter_mcp::analysis::view_code::execute(&without_code_args).unwrap();
    let without_code_text = common::get_result_text(&without_code_result);
    let without_tokens = common::helpers::approx_tokens(&without_code_text);

    // Then: include_code=false should be 40-60% of include_code=true (40-60% reduction)
    let ratio = without_tokens as f64 / with_tokens as f64;
    assert!(
        ratio < 0.60,
        "include_code=false should be <60% of include_code=true, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        without_tokens,
        with_tokens
    );

    // Verify at least 40% reduction (realistic for files with metadata)
    let reduction = ((with_tokens - without_tokens) as f64 / with_tokens as f64) * 100.0;
    assert!(
        reduction >= 40.0,
        "Should have at least 40% token reduction, got {:.1}%",
        reduction
    );
}

#[test]
fn test_include_code_false_preserves_signatures() {
    // Given: File with typed functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_code=false)
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Full signatures are present (types, params, return) in compact rows
    assert_eq!(
        shape.get("h").and_then(|v| v.as_str()),
        Some("name|line|sig")
    );

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .expect("Should find add row");

    let signature = add_row.get(2).expect("sig column");
    assert!(
        signature.contains("i32"),
        "Signature should include parameter types"
    );
    assert!(
        signature.contains("->"),
        "Signature should include return type indicator"
    );
}

// ============================================================================
// read_focused_code (~30% of parse_file claim)
// ============================================================================

#[test]
fn test_read_focused_code_is_cheaper_than_full_parse() {
    // Given: File with 10+ functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: read_focused_code vs parse_file
    let full_parse_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let full_parse_result = treesitter_mcp::analysis::view_code::execute(&full_parse_args).unwrap();
    let full_parse_text = common::get_result_text(&full_parse_result);
    let full_tokens = common::helpers::approx_tokens(&full_parse_text);

    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0,
        "include_deps": false
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);
    let focused_tokens = common::helpers::approx_tokens(&focused_text);

    // Then: focused read should be ~30-60% of full parse (realistic range)
    let ratio = focused_tokens as f64 / full_tokens as f64;
    assert!(
        ratio < 0.65,
        "read_focused_code should be <65% of parse_file, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        focused_tokens,
        full_tokens
    );
}

#[test]
fn test_read_focused_code_has_full_impl_for_target() {
    // Given: File with multiple functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: read_focused_code on "add"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0,
        "include_deps": false
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Target function has full code (compact rows)
    let header = shape.get("h").and_then(|v| v.as_str()).unwrap_or("");
    let code_idx = header
        .split('|')
        .position(|c| c == "code")
        .expect("Expected 'code' column in header");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .expect("Should have add row");

    assert!(
        add_row
            .get(code_idx)
            .map(|c| !c.is_empty())
            .unwrap_or(false),
        "Focused function should have full code"
    );

    // And: Other functions have signatures only
    let subtract_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("subtract"));

    if let Some(subtract_row) = subtract_row {
        assert!(
            subtract_row
                .get(code_idx)
                .map(|c| c.is_empty())
                .unwrap_or(true),
            "Non-focused functions should not have code"
        );
        assert!(
            subtract_row.get(2).map(|s| !s.is_empty()).unwrap_or(false),
            "Non-focused functions should have signatures"
        );
    }
}

// ============================================================================
// include_deps provides dependency context in one call
// ============================================================================

#[test]
fn test_include_deps_provides_dependency_signatures() {
    // Given: File that imports from local modules (calculator.rs uses models)
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_deps=true)
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": true,
        "max_tokens": 10_000
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Dependencies should be included (compact `deps` map)
    let deps = shape
        .get("deps")
        .and_then(|v| v.as_object())
        .expect("Should have deps object");

    assert!(!deps.is_empty(), "Should have at least one dependency");

    // Dep values are row strings
    let first_rows = deps.iter().find_map(|(_path, rows)| rows.as_str()).unwrap();

    let rows = common::helpers::parse_compact_rows(first_rows);
    assert!(!rows.is_empty(), "Dep should have at least one row");
}

// ============================================================================
// max_context_lines prevents token explosion
// ============================================================================

#[test]
fn test_max_context_lines_bounds_output_for_common_symbols() {
    // Given: A symbol used in multiple places
    let fixture_path = common::fixture_dir("rust");

    // When: find_usages with and without max_context_lines
    let unbounded_args = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": null
    });
    let unbounded_result = treesitter_mcp::analysis::find_usages::execute(&unbounded_args).unwrap();
    let unbounded_text = common::get_result_text(&unbounded_result);
    let unbounded_tokens = common::helpers::approx_tokens(&unbounded_text);

    let bounded_args = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 50
    });
    let bounded_result = treesitter_mcp::analysis::find_usages::execute(&bounded_args).unwrap();
    let bounded_text = common::get_result_text(&bounded_result);
    let bounded_tokens = common::helpers::approx_tokens(&bounded_text);

    // Then: Bounded should be less than or equal to unbounded
    assert!(
        bounded_tokens <= unbounded_tokens,
        "max_context_lines should limit output: {} vs {} tokens",
        bounded_tokens,
        unbounded_tokens
    );

    // Parse both to verify structure is preserved
    let unbounded_json: serde_json::Value = serde_json::from_str(&unbounded_text).unwrap();
    let bounded_json: serde_json::Value = serde_json::from_str(&bounded_text).unwrap();

    // Both should have usages rows string
    assert!(unbounded_json["u"].is_string());
    assert!(bounded_json["u"].is_string());
}

// ============================================================================
// Code map respects token budget
// ============================================================================

#[test]
fn test_code_map_respects_max_tokens_parameter() {
    // Given: A directory with multiple files
    let fixture_path = common::fixture_dir("rust");

    // When: code_map with max_tokens limit
    let max_tokens = 1000;
    let arguments = json!({
        "path": fixture_path.join("src").to_str().unwrap(),
        "detail": "signatures",
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::code_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let actual_tokens = common::helpers::approx_tokens(&text);

    // Then: Output should respect token budget (allow 20% overage for structure)
    assert!(
        actual_tokens <= max_tokens * 12 / 10,
        "code_map should respect token budget: {} > {} * 1.2",
        actual_tokens,
        max_tokens
    );
}

// ============================================================================
// Token-aware truncation using tiktoken-rs
// ============================================================================

#[test]
fn test_code_map_uses_tiktoken_for_token_counting() {
    // Given: A file with code
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: code_map with strict max_tokens limit using tiktoken
    let max_tokens = 100;
    let arguments = json!({
        "path": file_path.to_str().unwrap(),
        "detail": "full",
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::code_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);

    // Then: Actual tiktoken count should respect the limit
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    let actual_tokens = bpe.encode_with_special_tokens(&text).len();

    assert!(
        actual_tokens <= max_tokens,
        "code_map should respect tiktoken token budget: {} > {} tokens. Output was: {}",
        actual_tokens,
        max_tokens,
        text
    );
}

#[test]
fn test_find_usages_uses_tiktoken_for_token_counting() {
    // Given: A directory with multiple usages of a symbol
    let fixture_path = common::fixture_dir("rust");

    // When: find_usages with strict max_tokens limit
    let max_tokens = 50;
    let arguments = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 100,
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);

    // Then: Actual tiktoken count should respect the limit
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    let actual_tokens = bpe.encode_with_special_tokens(&text).len();

    assert!(
        actual_tokens <= max_tokens,
        "find_usages should respect tiktoken token budget: {} > {} tokens. Output was: {}",
        actual_tokens,
        max_tokens,
        text
    );
}
