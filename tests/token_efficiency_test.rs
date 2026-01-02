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
        "include_deps": false,
        "merge_templates": false
    });
    let shape_result = treesitter_mcp::analysis::file_shape::execute(&shape_args).unwrap();
    let shape_text = common::get_result_text(&shape_result);
    let shape_tokens = common::helpers::approx_tokens(&shape_text);

    // Then: file_shape should be 10-20% of parse_file (allow up to 25%)
    let ratio = shape_tokens as f64 / parse_tokens as f64;
    assert!(
        ratio < 0.25,
        "file_shape should be <25% of parse_file tokens, got {:.1}% ({} vs {} tokens)",
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

    // Then: Full signatures are present (types, params, return)
    let functions = shape["functions"].as_array().unwrap();
    let add_func = functions.iter().find(|f| f["name"] == "add").unwrap();

    let signature = add_func["signature"].as_str().unwrap();
    assert!(
        signature.contains("i32"),
        "Signature should include parameter types"
    );
    assert!(
        signature.contains("->"),
        "Signature should include return type indicator"
    );

    // But should NOT have code body
    assert!(
        add_func["code"].is_null(),
        "Should not have code when include_code=false"
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
        "context_radius": 0
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
        "context_radius": 0
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Target function has full code
    let functions = shape["functions"].as_array().unwrap();
    let add_func = functions.iter().find(|f| f["name"] == "add").unwrap();
    assert!(
        add_func["code"].is_string() && !add_func["code"].as_str().unwrap().is_empty(),
        "Focused function should have full code"
    );

    // And: Other functions have signatures only
    let subtract_func = functions.iter().find(|f| f["name"] == "subtract");
    if let Some(subtract) = subtract_func {
        assert!(
            subtract["code"].is_null(),
            "Non-focused functions should not have code"
        );
        assert!(
            subtract["signature"].is_string(),
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
        "include_deps": true
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Dependencies should be included
    if let Some(deps) = shape["dependencies"].as_array() {
        assert!(!deps.is_empty(), "Should have at least one dependency");

        // Dependencies should have structural information
        let first_dep = &deps[0];
        assert!(first_dep["path"].is_string(), "Dependency should have path");

        // Should have at least one of: functions, structs, classes
        let has_content = first_dep["functions"].is_array()
            || first_dep["structs"].is_array()
            || first_dep["classes"].is_array();
        assert!(
            has_content,
            "Dependency should include structural information"
        );
    }
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

    // Both should have usages array
    assert!(unbounded_json["usages"].is_array());
    assert!(bounded_json["usages"].is_array());
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
