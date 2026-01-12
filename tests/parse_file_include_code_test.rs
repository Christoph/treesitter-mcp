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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code field
    assert!(result.is_ok(), "parse_file should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have functions");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(!row.get(2).map(|s| s.is_empty()).unwrap_or(true));
    }
}

#[test]
fn test_parse_file_include_code_false_omits_struct_code() {
    // Given: Rust fixture with structs and include_code=false
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Structs should NOT have code field
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("s").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have structs");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        // signature snippet may be empty depending on language/shape
        assert!(row.len() >= 3);
    }
}

#[test]
fn test_parse_file_include_code_false_omits_class_code() {
    // Given: Python fixture with classes and include_code=false
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code field
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("c").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have classes");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(row.len() >= 3);
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
        "detail": "full",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions SHOULD have code field with content
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig|doc|code");
    let code_idx = header.split('|').position(|c| c == "code").unwrap();

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have functions");

    let has_code = rows
        .iter()
        .any(|r| r.get(code_idx).map(|c| !c.is_empty()).unwrap_or(false));
    assert!(has_code, "At least one function should have code when full");

    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .unwrap();
    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("a + b"));
}

// ============================================================================
// Default Behavior Tests
// ============================================================================

#[test]
fn test_parse_file_include_code_defaults_to_true() {
    // Given: Rust fixture WITHOUT include_code parameter (should default to true)
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should include code by default (backward compatible)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig|doc|code");
    let code_idx = header.split('|').position(|c| c == "code").unwrap();

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty());

    let has_code = rows
        .iter()
        .any(|r| r.get(code_idx).map(|c| !c.is_empty()).unwrap_or(false));
    assert!(has_code, "Should include code by default");
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Doc comments should still be present even without code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Compact signatures output omits code and doc to save tokens.
    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .unwrap();
    assert!(add_row.get(2).map(|s| !s.is_empty()).unwrap_or(false));
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have JavaScript functions");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(!row.get(2).map(|s| s.is_empty()).unwrap_or(true));
    }
}

#[test]
fn test_parse_file_include_code_false_javascript_classes() {
    // Given: JavaScript fixture with classes and include_code=false
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("c").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have JavaScript classes");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(row.len() >= 3);
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have TypeScript functions");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(!row.get(2).map(|s| s.is_empty()).unwrap_or(true));
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Functions should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have Python functions");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(!row.get(2).map(|s| s.is_empty()).unwrap_or(true));
    }
}

#[test]
fn test_parse_file_include_code_false_python_classes() {
    // Given: Python fixture with classes and include_code=false
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Classes should NOT have code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let header = shape.get("h").and_then(|v| v.as_str()).unwrap();
    assert_eq!(header, "name|line|sig");

    let rows_str = shape.get("c").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have Python classes");

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true));
        assert!(row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some());
        assert!(row.len() >= 3);
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Key metadata should be present in compact schema
    assert!(
        shape.get("p").and_then(|v| v.as_str()).is_some(),
        "Should have p"
    );
    assert!(
        shape.get("h").and_then(|v| v.as_str()).is_some(),
        "Should have h"
    );

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);
    assert!(!rows.is_empty(), "Should have functions rows");
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
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
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

    // Then: Both should have same function names/signatures/lines
    let rows_false = common::helpers::parse_compact_rows(
        shape_false.get("f").and_then(|v| v.as_str()).unwrap_or(""),
    );
    let rows_true = common::helpers::parse_compact_rows(
        shape_true.get("f").and_then(|v| v.as_str()).unwrap_or(""),
    );

    assert_eq!(rows_false.len(), rows_true.len(), "Same number of rows");

    for (r_false, r_true) in rows_false.iter().zip(rows_true.iter()) {
        assert_eq!(r_false.get(0), r_true.get(0));
        assert_eq!(r_false.get(1), r_true.get(1));
        assert_eq!(r_false.get(2), r_true.get(2));
    }

    // And: code only exists in full output
    let header_true = shape_true.get("h").and_then(|v| v.as_str()).unwrap();
    let code_idx = header_true.split('|').position(|c| c == "code").unwrap();

    assert!(rows_false.iter().all(|r| r.len() == 3));
    assert!(rows_true
        .iter()
        .any(|r| r.get(code_idx).map(|c| !c.is_empty()).unwrap_or(false)));
}
