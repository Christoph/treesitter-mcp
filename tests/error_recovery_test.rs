//! Error recovery tests - verifying errors help LLMs self-correct
//!
//! These tests ensure error messages are actionable for LLM agents.

mod common;

use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// File Not Found Errors
// ============================================================================

#[test]
fn test_parse_file_not_found_error_is_actionable() {
    // When: parse_file on non-existent file
    let arguments = json!({
        "file_path": "/nonexistent/path/to/file.rs",
        "include_code": true,
        "include_deps": false
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Error message should be clear about file not found
    assert!(result.is_err(), "Should error on non-existent file");

    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("no such file")
            || err_lower.contains("does not exist"),
        "Error should indicate file not found, got: {}",
        err
    );
}

#[test]
fn test_file_shape_not_found_error_is_actionable() {
    // When: file_shape on non-existent file
    let arguments = json!({
        "file_path": "/nonexistent/path/to/file.rs",
        "include_deps": false,
        "merge_templates": false
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should error clearly
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("not found")
            || err_lower.contains("no such file")
            || err_lower.contains("does not exist"),
        "Error should indicate file not found, got: {}",
        err
    );
}

// ============================================================================
// Invalid Position Errors
// ============================================================================

#[test]
fn test_get_context_line_out_of_range_handles_gracefully() {
    // Given: Valid file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: get_context with line way beyond file length
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 9999,
        "column": 1
    });

    let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments);

    // Then: Should either error clearly or return minimal context (not panic)
    if let Ok(call_result) = result {
        let text = common::get_result_text(&call_result);
        let context: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Compact schema is acceptable if it parses
        assert!(context.is_object());
    } else {
        // If it errors, that's also acceptable behavior
        assert!(result.is_err());
    }
}

#[test]
fn test_get_node_at_position_out_of_range_handles_gracefully() {
    // Given: Valid file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: get_node_at_position with invalid position
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 9999,
        "column": 9999,
        "ancestor_levels": 3
    });

    let result = treesitter_mcp::analysis::symbol_at_line::execute(&arguments);

    // Then: Should handle gracefully (not panic)
    // Either returns error or minimal node info
    if result.is_ok() {
        let text = common::get_result_text(&result.unwrap());
        // Should be valid JSON at minimum
        let _node: serde_json::Value = serde_json::from_str(&text).unwrap();
    }
}

// ============================================================================
// Empty Results (Not Errors)
// ============================================================================

#[test]
fn test_find_usages_nonexistent_symbol_returns_empty() {
    // Given: Valid file with known symbols
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: find_usages for symbol that doesn't exist
    let arguments = json!({
        "symbol": "nonexistent_symbol_xyz_12345",
        "path": file_path.to_str().unwrap(),
        "context_lines": 2
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: Should not error, but return empty array
    assert!(result.is_ok(), "Should not error on nonexistent symbol");

    let text = common::get_result_text(&result.unwrap());
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows = common::helpers::find_usages_rows(&usages);
    assert_eq!(
        rows.len(),
        0,
        "Should return empty rows for nonexistent symbol"
    );
    assert_eq!(
        usages["sym"], "nonexistent_symbol_xyz_12345",
        "Should echo back the searched symbol"
    );
}

#[test]
fn test_code_map_empty_directory_returns_empty_files() {
    // Given: Empty directory (no source files)
    let dir = TempDir::new().unwrap();

    // When: code_map on empty directory
    let arguments = json!({
        "path": dir.path().to_str().unwrap(),
        "detail": "minimal",
        "max_tokens": 2000
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Should succeed with empty files array
    assert!(result.is_ok(), "Should not error on empty directory");

    let text = common::get_result_text(&result.unwrap());
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(common::helpers::code_map_files(&map).is_empty());
}

#[test]
fn test_read_focused_code_nonexistent_symbol_returns_file_shape() {
    // Given: Valid file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: read_focused_code on symbol that doesn't exist
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "nonexistent_function_xyz",
        "context_radius": 0
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed but return file shape (no focused function)
    // This is acceptable behavior - LLM can see the file has no such symbol
    assert!(
        result.is_ok(),
        "Should handle gracefully when symbol not found"
    );

    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Compact schema: should at least include a symbol table.
    assert!(shape.get("h").and_then(|v| v.as_str()).is_some());
    assert!(
        shape.get("f").is_some() || shape.get("s").is_some() || shape.get("c").is_some(),
        "Expected at least one of f/s/c in view_code output"
    );
}

// ============================================================================
// Git-related Errors
// ============================================================================

#[test]
fn test_parse_diff_not_git_repo_error_is_clear() {
    // Given: File not in a git repo
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    // When: parse_diff on non-git file
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);

    // Then: Error should mention git
    assert!(result.is_err(), "Should error when not in git repo");

    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("git") || err_lower.contains("repository"),
        "Error should mention git or repository, got: {}",
        err
    );
}

#[test]
fn test_affected_by_diff_not_git_repo_error_is_clear() {
    // Given: File not in a git repo
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    // When: affected_by_diff on non-git file
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD",
        "scope": dir.path().to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::diff::execute_affected_by_diff(&arguments);

    // Then: Should error mentioning git
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("git") || err_lower.contains("repository"),
        "Error should mention git requirement"
    );
}

// ============================================================================
// Query Pattern Errors
// ============================================================================

#[test]
fn test_query_pattern_invalid_syntax_error_is_helpful() {
    // Given: Valid file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: query_pattern with malformed S-expression
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "(this is not valid s-expression syntax",
        "context_lines": 2
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: Should error with explanation
    assert!(result.is_err(), "Should error on invalid query syntax");

    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("query") || err_lower.contains("syntax") || err_lower.contains("parse"),
        "Error should explain query syntax issue, got: {}",
        err
    );
}

#[test]
fn test_query_pattern_empty_query_returns_empty_matches() {
    // Given: Valid file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: query_pattern with empty query
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "query": "",
        "context_lines": 2
    });

    let result = treesitter_mcp::analysis::query_pattern::execute(&arguments);

    // Then: May error or return empty matches - both are acceptable
    // If it succeeds, should have empty matches
    if result.is_ok() {
        let text = common::get_result_text(&result.unwrap());
        let matches: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Empty query likely returns no matches
        assert!(matches["m"].is_string());
    }
}

// ============================================================================
// Unsupported Language Handling
// ============================================================================

#[test]
fn test_parse_file_unsupported_extension_error() {
    // Given: File with unsupported extension
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.xyz");
    fs::write(&file_path, "some content").unwrap();

    // When: parse_file on unsupported extension
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": false
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should error (unsupported language)
    assert!(
        result.is_err(),
        "Should error on unsupported file extension"
    );

    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("unsupported")
            || err_lower.contains("language")
            || err_lower.contains("unknown"),
        "Error should indicate unsupported language/extension, got: {}",
        err
    );
}

// ============================================================================
// Directory vs File Errors
// ============================================================================

#[test]
fn test_parse_file_on_directory_gives_clear_error() {
    // Given: A directory path
    let dir_path = common::fixture_dir("rust");

    // When: parse_file on a directory (not a file)
    let arguments = json!({
        "file_path": dir_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": false
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should error clearly
    assert!(
        result.is_err(),
        "Should error when given directory instead of file"
    );

    let err = result.unwrap_err().to_string();
    let err_lower = err.to_lowercase();
    assert!(
        err_lower.contains("directory")
            || err_lower.contains("not a file")
            || err_lower.contains("is a dir"),
        "Error should indicate directory vs file issue, got: {}",
        err
    );
}
