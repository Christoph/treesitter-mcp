use serde_json::json;

mod common;

fn header_index(shape: &serde_json::Value, col: &str) -> usize {
    shape
        .get("h")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .split('|')
        .position(|c| c == col)
        .unwrap_or_else(|| panic!("Missing column '{col}' in header"))
}

fn table_rows(shape: &serde_json::Value, key: &str) -> Vec<Vec<String>> {
    let rows_str = shape.get(key).and_then(|v| v.as_str()).unwrap_or("");
    common::helpers::parse_compact_rows(rows_str)
}

fn find_row_by_name<'a>(rows: &'a [Vec<String>], name: &str) -> &'a Vec<String> {
    rows.iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some(name))
        .unwrap_or_else(|| panic!("Missing row for '{name}'"))
}

// ============================================================================
// Rust Tests
// ============================================================================

#[test]
fn test_parse_file_extracts_function_signatures_and_code() {
    // Given: Rust fixture with functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    // When: view_code is called
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Returns JSON with compact tables
    assert!(result.is_ok(), "view_code should succeed");
    let call_result = result.unwrap();
    assert!(
        !call_result.is_error.unwrap_or(false),
        "Should not be an error"
    );

    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have relative path
    let p = shape.get("p").and_then(|v| v.as_str()).unwrap();
    assert!(p.contains("calculator.rs"));

    // Full detail by default
    assert_eq!(
        shape.get("h").and_then(|v| v.as_str()),
        Some("name|line|sig|doc|code")
    );

    // Should have functions table
    let rows = table_rows(&shape, "f");
    assert!(rows.len() >= 5);

    // Check for specific functions using helper (supports compact)
    common::helpers::assert_has_function(&shape, "add");
    common::helpers::assert_has_function(&shape, "subtract");
    common::helpers::assert_has_function(&shape, "multiply");
    common::helpers::assert_has_function(&shape, "divide");

    // Verify the actual code is included using helper
    common::helpers::assert_function_code_contains(&shape, "add", "a + b");
    common::helpers::assert_function_code_contains(&shape, "add", "pub fn add");
}

#[test]
fn test_parse_file_extracts_struct_definitions_and_fields() {
    // Given: Rust fixture with structs
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(
        shape.get("h").and_then(|v| v.as_str()),
        Some("name|line|sig|doc|code")
    );

    let rows = table_rows(&shape, "s");
    assert!(rows.len() >= 2); // Calculator, Point

    let code_idx = header_index(&shape, "code");

    // Check for Calculator struct
    let calc_row = find_row_by_name(&rows, "Calculator");
    let code = calc_row.get(code_idx).unwrap();
    assert!(code.contains("Calculator"));
    assert!(code.contains("pub value: i32"));
}

#[test]
fn test_parse_file_rust_docs() {
    // Given: Rust fixture with doc comments
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "f");
    let add_row = find_row_by_name(&rows, "add");

    let doc_idx = header_index(&shape, "doc");
    let code_idx = header_index(&shape, "code");

    let doc = add_row.get(doc_idx).unwrap();
    assert!(doc.contains("Adds two numbers"));

    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("a + b"));
}

#[test]
fn test_parse_file_extracts_import_statements() {
    // Given: Rust fixture with imports
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Imports are rows in `im`.
    let rows = table_rows(&shape, "im");
    assert!(!rows.is_empty());

    let has_fmt_import = rows
        .iter()
        .any(|r| r.get(1).map(|s| s.contains("std::fmt")).unwrap_or(false));
    assert!(has_fmt_import);
}

// ============================================================================
// Python Tests
// ============================================================================

#[test]
fn test_parse_file_handles_python_function_definitions() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let code_idx = header_index(&shape, "code");

    let rows = table_rows(&shape, "f");
    assert!(rows.len() >= 4);

    let add_row = find_row_by_name(&rows, "add");
    assert!(add_row.get(2).unwrap().contains("def add"));

    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("return a + b"));
    assert!(code.contains("def add"));
}

#[test]
fn test_parse_file_handles_python_class_definitions() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "c");
    assert!(rows.len() >= 2);

    let code_idx = header_index(&shape, "code");
    let calc_row = find_row_by_name(&rows, "Calculator");

    let code = calc_row.get(code_idx).unwrap();
    assert!(code.contains("class Calculator"));
    assert!(code.contains("def __init__"));
}

// ============================================================================
// JavaScript Tests
// ============================================================================

#[test]
fn test_parse_file_handles_javascript_function_declarations() {
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "f");
    assert!(rows.len() >= 4);

    let code_idx = header_index(&shape, "code");
    if let Some(add_row) = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
    {
        let code = add_row.get(code_idx).unwrap();
        assert!(code.contains("return") || code.contains("a + b"));
    }
}

#[test]
fn test_parse_file_javascript_classes() {
    let file_path = common::fixture_path("javascript", "calculator.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "c");
    assert!(rows.len() >= 2); // Calculator, Point
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_parse_file_typescript_with_types() {
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let code_idx = header_index(&shape, "code");

    let rows = table_rows(&shape, "f");
    let add_row = find_row_by_name(&rows, "add");
    assert!(add_row.get(2).unwrap().contains("number"));

    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("return") || code.contains("a + b"));
}

#[test]
fn test_parse_file_handles_typescript_interface_definitions() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Interfaces are in `i` table.
    assert!(shape.get("i").is_some() || shape.get("c").is_some());
}

// ============================================================================
// Error Cases
// ============================================================================

#[test]
fn test_parse_file_nonexistent_file() {
    let arguments = json!({"file_path": "/nonexistent/file.rs"});
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);
    assert!(result.is_err(), "Should return error for non-existent file");
}

#[test]
fn test_parse_file_unsupported_extension() {
    use std::fs;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "plain text").unwrap();

    let arguments = json!({"file_path": file_path.to_str().unwrap()});
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

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
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "f");
    let add_row = find_row_by_name(&rows, "add");

    let code_idx = header_index(&shape, "code");
    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("pub fn add(a: i32, b: i32) -> i32"));
    assert!(code.contains("a + b"));

    let sub_row = find_row_by_name(&rows, "subtract");
    let sub_code = sub_row.get(code_idx).unwrap();
    assert!(sub_code.contains("a - b"));
}

#[test]
fn test_parse_file_python_code_matches_fixture() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = table_rows(&shape, "f");
    let add_row = find_row_by_name(&rows, "add");

    let code_idx = header_index(&shape, "code");

    let code = add_row.get(code_idx).unwrap();
    assert!(code.contains("def add(a, b):"));
    assert!(code.contains("return a + b"));

    // Some languages don't extract doc strings consistently into the `doc` column;
    // the full `code` block should still contain the docstring.
    assert!(code.contains("Adds two numbers together"));
}
