mod common;

use serde_json::json;
use std::path::PathBuf;

fn count_symbols(file: &serde_json::Value) -> usize {
    let f = file
        .get("f")
        .and_then(|v| v.as_str())
        .map(|s| if s.is_empty() { 0 } else { s.lines().count() })
        .unwrap_or(0);

    let s = file
        .get("s")
        .and_then(|v| v.as_str())
        .map(|s| if s.is_empty() { 0 } else { s.lines().count() })
        .unwrap_or(0);

    let c = file
        .get("c")
        .and_then(|v| v.as_str())
        .map(|s| if s.is_empty() { 0 } else { s.lines().count() })
        .unwrap_or(0);

    f + s + c
}

// ============================================================================
// Detail Level Tests
// ============================================================================

#[test]
fn test_code_map_provides_minimal_overview_with_names_only() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "minimal"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let (_path, file_obj) = common::helpers::code_map_find_file(&map, "calculator.rs");
    assert_eq!(file_obj["h"], "name|line");

    let functions = common::helpers::compact_table_get_rows(file_obj, "f");
    let add_fn = functions
        .iter()
        .find(|row| row.first().map(|v| v.as_str()) == Some("add"))
        .unwrap();

    assert_eq!(add_fn[0], "add");
    assert_eq!(add_fn.len(), 2);
}

#[test]
fn test_code_map_includes_signatures_at_medium_detail() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "signatures"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let (_path, file_obj) = common::helpers::code_map_find_file(&map, "calculator.rs");
    assert_eq!(file_obj["h"], "name|line|sig");

    let functions = common::helpers::compact_table_get_rows(file_obj, "f");
    let add_fn = functions
        .iter()
        .find(|row| row.first().map(|v| v.as_str()) == Some("add"))
        .unwrap();

    assert!(add_fn.len() >= 3);
    assert!(add_fn[2].contains("pub fn add"));
    assert!(add_fn[2].contains("i32"));
}

#[test]
fn test_code_map_includes_full_details_with_docs() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "full",
        "max_tokens": 10000
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let (_path, file_obj) = common::helpers::code_map_find_file(&map, "calculator.rs");
    assert_eq!(file_obj["h"], "name|line|sig|doc|code");

    let functions = common::helpers::compact_table_get_rows(file_obj, "f");
    let add_fn = functions
        .iter()
        .find(|row| row.first().map(|v| v.as_str()) == Some("add"))
        .unwrap();

    assert!(add_fn.len() >= 5);
    assert!(add_fn[2].contains("pub fn add"));
    assert!(add_fn[3].contains("Adds two numbers"));
    assert!(add_fn[4].contains("a + b"));
    assert!(add_fn[4].contains("pub fn add"));
}

// ============================================================================
// Multi-Language Tests
// ============================================================================

#[test]
fn test_code_map_python_project() {
    let dir_path = common::fixture_dir("python");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = common::helpers::code_map_files(&map);
    assert!(files.len() >= 2);

    let (_path, file_obj) = common::helpers::code_map_find_file(&map, "calculator.py");
    let classes = common::helpers::compact_table_get_rows(file_obj, "c");
    assert!(classes.len() >= 2);

    let functions = common::helpers::compact_table_get_rows(file_obj, "f");
    let has_add = functions
        .iter()
        .any(|row| row.first() == Some(&"add".to_string()));
    assert!(has_add);
}

#[test]
fn test_code_map_javascript_project() {
    let dir_path = common::fixture_dir("javascript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = common::helpers::code_map_files(&map);
    assert!(files.len() >= 2);
}

#[test]
fn test_code_map_typescript_project() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = common::helpers::code_map_files(&map);
    assert!(files.len() >= 2);
}

// ============================================================================
// Feature Tests
// ============================================================================

#[test]
fn test_code_map_filters_files_by_glob_pattern() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "pattern": "*.rs"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    for (path, _) in common::helpers::code_map_files(&map) {
        assert!(path.ends_with(".rs"));
    }
}

#[test]
fn test_code_map_respects_token_budget_limit() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "max_tokens": 500,
        "detail": "full"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    // If output is large, we expect truncation marker.
    if text.len() > 500 * 6 {
        assert_eq!(map["@"]["t"].as_bool(), Some(true));
    }
}

#[test]
fn test_code_map_handles_single_file_analysis() {
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = common::helpers::code_map_files(&map);
    assert_eq!(files.len(), 1);
    assert!(files[0].0.contains("calculator.rs"));
}

#[test]
fn test_code_map_skips_hidden_and_vendor() {
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    for (path, _) in common::helpers::code_map_files(&map) {
        assert!(!path.contains("/target/"));
        assert!(!path.contains("/.git/"));
        assert!(!path.contains("/node_modules/"));
    }
}

// ============================================================================
// Token-Aware Truncation Tests
// ============================================================================

#[test]
fn test_code_map_truncates_intelligently_with_token_budget() {
    let dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("complex_rust_service");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "max_tokens": 50,
        "detail": "full"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());

    let approx_tokens = text.len() / 4;
    assert!(approx_tokens <= 75);

    let map: serde_json::Value = serde_json::from_str(&text).unwrap();
    let files = common::helpers::code_map_files(&map);
    assert!(!files.is_empty());

    if approx_tokens >= 40 {
        assert_eq!(map["@"]["t"].as_bool(), Some(true));
    }
}

#[test]
fn test_code_map_symbol_counts_are_reasonable() {
    let dir_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("complex_rust_service");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "signatures"
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    assert!(result.is_ok());
    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = common::helpers::code_map_files(&map);
    assert!(!files.is_empty());

    let max_symbols = files
        .iter()
        .map(|(_, v)| count_symbols(v))
        .max()
        .unwrap_or(0);

    assert!(max_symbols > 0);
}
