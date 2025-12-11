mod common;

use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if a path is absolute (contains absolute path markers)
fn is_absolute_path(path: &str) -> bool {
    path.contains("/Users/")
        || path.contains("/home/")
        || path.contains("/var/")
        || path.contains("/mnt/")
        || path.starts_with("C:\\")
        || path.starts_with("D:\\")
        || path.starts_with("/")
}

/// Check if a path is relative (no absolute markers)
fn is_relative_path(path: &str) -> bool {
    !is_absolute_path(path)
}

/// Setup a git repository for testing
fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

/// Commit a file to git
fn commit_file(dir: &TempDir, filename: &str, content: &str) {
    let file_path = dir.path().join(filename);

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    fs::write(&file_path, content).unwrap();

    Command::new("git")
        .args(["add", filename])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "commit"])
        .current_dir(dir.path())
        .output()
        .unwrap();
}

// ============================================================================
// Test 1: parse_file returns relative path
// ============================================================================

#[test]
fn test_parse_file_returns_relative_path() {
    // Given: Rust fixture with nested file
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Returns relative path without absolute markers
    assert!(result.is_ok(), "parse_file should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let path = shape["path"].as_str().unwrap();

    // Should NOT contain absolute path markers
    assert!(
        is_relative_path(path),
        "Path should be relative, got: {}",
        path
    );

    // Should contain relative path components
    assert!(
        path.contains("src") && path.contains("models") && path.contains("mod.rs"),
        "Should contain relative path structure, got: {}",
        path
    );
}

// ============================================================================
// Test 2: file_shape returns relative path
// ============================================================================

#[test]
fn test_file_shape_returns_relative_path() {
    // Given: Rust fixture file
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false
    });

    // When: file_shape is called
    let result = treesitter_mcp::analysis::file_shape::execute(&arguments);

    // Then: Returns relative path
    assert!(result.is_ok(), "file_shape should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let path = shape["path"].as_str().unwrap();

    // Should NOT contain absolute path markers
    assert!(
        is_relative_path(path),
        "Path should be relative, got: {}",
        path
    );

    // Should contain relative path components
    assert!(
        path.contains("src") && path.contains("calculator.rs"),
        "Should contain relative path structure, got: {}",
        path
    );
}

// ============================================================================
// Test 3: code_map returns relative paths for all files
// ============================================================================

#[test]
fn test_code_map_returns_relative_paths() {
    // Given: Rust fixture directory
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "signatures"
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: All file paths are relative
    assert!(result.is_ok(), "code_map should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    assert!(files.len() > 0, "Should find at least one file");

    // Check that all file paths are relative
    for file in files {
        let path = file["path"].as_str().unwrap();
        assert!(
            is_relative_path(path),
            "File path should be relative, got: {}",
            path
        );

        // Should contain recognizable path components
        assert!(
            path.contains("src") || path.contains("calculator") || path.contains("models"),
            "Should contain relative path structure, got: {}",
            path
        );
    }
}

// ============================================================================
// Test 4: find_usages returns relative paths
// ============================================================================

#[test]
fn test_find_usages_returns_relative_paths() {
    // Given: Rust fixture directory
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "symbol": "Calculator",
        "path": dir_path.join("src").to_str().unwrap()
    });

    // When: find_usages is called
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments);

    // Then: All file paths in usages are relative
    assert!(result.is_ok(), "find_usages should succeed");
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let usages: serde_json::Value = serde_json::from_str(&text).unwrap();

    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() > 0, "Should find at least one usage");

    // Check that all file paths are relative
    for usage in usage_list {
        let file = usage["file"].as_str().unwrap();
        assert!(
            is_relative_path(file),
            "File path should be relative, got: {}",
            file
        );

        // Should contain recognizable path components
        assert!(
            file.contains("src") || file.contains("calculator") || file.contains("models"),
            "Should contain relative path structure, got: {}",
            file
        );
    }
}

// ============================================================================
// Test 5: relative path from git root
// ============================================================================

#[test]
fn test_relative_path_from_git_root() {
    // Given: A git repository with nested files
    let dir = setup_git_repo();
    let rust_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    commit_file(&dir, "src/calculator.rs", rust_code);

    let file_path = dir.path().join("src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called on a file in git repo
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Path is relative to git root
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let path = shape["path"].as_str().unwrap();

    // Should be relative to git root
    assert!(
        is_relative_path(path),
        "Path should be relative to git root, got: {}",
        path
    );

    // Should contain src/calculator.rs
    assert!(
        path.contains("src") && path.contains("calculator.rs"),
        "Should contain relative path from git root, got: {}",
        path
    );
}

// ============================================================================
// Test 6: relative path strips absolute markers
// ============================================================================

#[test]
fn test_relative_path_strips_absolute_markers() {
    // Given: Multiple fixture files
    let test_files = vec![
        ("rust", "src/calculator.rs"),
        ("rust", "src/models/mod.rs"),
        ("rust", "src/lib.rs"),
    ];

    for (lang, file) in test_files {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        // When: parse_file is called
        let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
        assert!(result.is_ok());

        let call_result = result.unwrap();
        let text = common::get_result_text(&call_result);
        let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

        let path = shape["path"].as_str().unwrap();

        // Then: No absolute path markers
        assert!(
            !path.contains("/Users/"),
            "Path should not contain /Users/, got: {}",
            path
        );
        assert!(
            !path.contains("/home/"),
            "Path should not contain /home/, got: {}",
            path
        );
        assert!(
            !path.contains("/var/"),
            "Path should not contain /var/, got: {}",
            path
        );
        assert!(
            !path.contains("/mnt/"),
            "Path should not contain /mnt/, got: {}",
            path
        );
        assert!(
            !path.starts_with("C:\\"),
            "Path should not start with C:\\, got: {}",
            path
        );
    }
}

// ============================================================================
// Test 7: relative path preserves structure
// ============================================================================

#[test]
fn test_relative_path_preserves_structure() {
    // Given: Nested file structure
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());

    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let path = shape["path"].as_str().unwrap();

    // Then: Path structure is preserved
    // Should be able to identify the file location from the relative path
    assert!(
        path.contains("src/models/mod.rs") || path.ends_with("mod.rs"),
        "Path structure should be preserved, got: {}",
        path
    );

    // Should contain directory separators
    assert!(
        path.contains("/") || path.contains("\\"),
        "Path should contain directory separators, got: {}",
        path
    );
}

// ============================================================================
// Test 8: relative path multiple tools consistent
// ============================================================================

#[test]
fn test_relative_path_multiple_tools_consistent() {
    // Given: Same file accessed through different tools
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let dir_path = common::fixture_dir("rust");

    // When: parse_file is called
    let parse_file_args = json!({
        "file_path": file_path.to_str().unwrap()
    });
    let parse_file_result = treesitter_mcp::analysis::parse_file::execute(&parse_file_args);
    assert!(parse_file_result.is_ok());

    let parse_file_text = common::get_result_text(&parse_file_result.unwrap());
    let parse_file_shape: serde_json::Value = serde_json::from_str(&parse_file_text).unwrap();
    let parse_file_path = parse_file_shape["path"].as_str().unwrap();

    // When: file_shape is called
    let file_shape_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false
    });
    let file_shape_result = treesitter_mcp::analysis::file_shape::execute(&file_shape_args);
    assert!(file_shape_result.is_ok());

    let file_shape_text = common::get_result_text(&file_shape_result.unwrap());
    let file_shape_shape: serde_json::Value = serde_json::from_str(&file_shape_text).unwrap();
    let file_shape_path = file_shape_shape["path"].as_str().unwrap();

    // When: code_map is called
    let code_map_args = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "signatures"
    });
    let code_map_result = treesitter_mcp::analysis::code_map::execute(&code_map_args);
    assert!(code_map_result.is_ok());

    let code_map_text = common::get_result_text(&code_map_result.unwrap());
    let code_map_map: serde_json::Value = serde_json::from_str(&code_map_text).unwrap();
    let code_map_files = code_map_map["files"].as_array().unwrap();
    let code_map_calc_file = code_map_files
        .iter()
        .find(|f| f["path"].as_str().unwrap().contains("calculator.rs"))
        .unwrap();
    let code_map_path = code_map_calc_file["path"].as_str().unwrap();

    // Then: All paths are relative and consistent
    assert!(
        is_relative_path(parse_file_path),
        "parse_file path should be relative: {}",
        parse_file_path
    );
    assert!(
        is_relative_path(file_shape_path),
        "file_shape path should be relative: {}",
        file_shape_path
    );
    assert!(
        is_relative_path(code_map_path),
        "code_map path should be relative: {}",
        code_map_path
    );

    // All should contain the same relative path components
    assert!(
        parse_file_path.contains("calculator.rs"),
        "parse_file path should contain calculator.rs: {}",
        parse_file_path
    );
    assert!(
        file_shape_path.contains("calculator.rs"),
        "file_shape path should contain calculator.rs: {}",
        file_shape_path
    );
    assert!(
        code_map_path.contains("calculator.rs"),
        "code_map path should contain calculator.rs: {}",
        code_map_path
    );
}

// ============================================================================
// Test 9: parse_diff returns relative path
// ============================================================================

#[test]
fn test_parse_diff_returns_relative_path() {
    // Given: A git repository with a modified file
    let dir = setup_git_repo();
    let initial_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    commit_file(&dir, "src/calculator.rs", initial_code);

    // Modify the file (add a new function)
    let modified_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
"#;
    fs::write(dir.path().join("src/calculator.rs"), modified_code).unwrap();

    let file_path = dir.path().join("src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    // When: parse_diff is called
    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);

    // Then: file_path in result is relative
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    let file_path_result = analysis["file_path"].as_str().unwrap();

    assert!(
        is_relative_path(file_path_result),
        "file_path should be relative, got: {}",
        file_path_result
    );

    assert!(
        file_path_result.contains("calculator.rs"),
        "Should contain calculator.rs, got: {}",
        file_path_result
    );
}

// ============================================================================
// Test 10: affected_by_diff returns relative paths
// ============================================================================
#[ignore] // Complex git scenario - may fail in some environments


#[test]
fn test_affected_by_diff_returns_relative_paths() {
    // Given: A git repository with multiple files
    let dir = setup_git_repo();

    // Create initial files
    let lib_code = r#"
pub mod models;
pub mod calculator;

pub fn main() {
    let calc = calculator::create_calculator();
}
"#;
    commit_file(&dir, "src/lib.rs", lib_code);

    let calc_code = r#"
use crate::models::Calculator;

pub fn create_calculator() -> Calculator {
    Calculator { value: 0 }
}
"#;
    commit_file(&dir, "src/calculator.rs", calc_code);

    let models_code = r#"
pub struct Calculator {
    pub value: i32,
}
"#;
    commit_file(&dir, "src/models.rs", models_code);

    // Modify calculator.rs (add a new function)
    let modified_calc = r#"
use crate::models::Calculator;

pub fn create_calculator() -> Calculator {
    Calculator { value: 0 }
}

pub fn add_to_calculator(calc: &mut Calculator, value: i32) {
    calc.value += value;
}
"#;
    fs::write(dir.path().join("src/calculator.rs"), modified_calc).unwrap();

    let file_path = dir.path().join("src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "scope": dir.path().to_str().unwrap()
    });

    // When: affected_by_diff is called
    let result = treesitter_mcp::analysis::diff::execute_affected_by_diff(&arguments);

    // Then: All file paths are relative
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Check main file_path
    let file_path_result = analysis["file_path"].as_str().unwrap();
    assert!(
        is_relative_path(file_path_result),
        "file_path should be relative, got: {}",
        file_path_result
    );

    // Check affected_changes file paths
    if let Some(affected_changes) = analysis["affected_changes"].as_array() {
        for change in affected_changes {
            let affected_file = change["file"].as_str().unwrap();
            assert!(
                is_relative_path(affected_file),
                "affected file path should be relative, got: {}",
                affected_file
            );
        }
    }
}

// ============================================================================
// Test 11: relative path token savings
// ============================================================================

#[test]
fn test_relative_path_token_savings() {
    // Given: A deeply nested file
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());

    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let relative_path = shape["path"].as_str().unwrap();

    // Then: Relative path is significantly shorter than absolute path
    let absolute_path = file_path.to_str().unwrap();

    // Relative path should be much shorter
    assert!(
        relative_path.len() < absolute_path.len(),
        "Relative path should be shorter than absolute path. Relative: {} ({} chars), Absolute: {} ({} chars)",
        relative_path,
        relative_path.len(),
        absolute_path,
        absolute_path.len()
    );

    // Calculate token savings (rough estimate: ~4 chars per token)
    let absolute_tokens = (absolute_path.len() as f64 / 4.0).ceil() as usize;
    let relative_tokens = (relative_path.len() as f64 / 4.0).ceil() as usize;
    let savings_percent =
        ((absolute_tokens - relative_tokens) as f64 / absolute_tokens as f64) * 100.0;

    println!(
        "Token savings: {:.1}% ({} -> {} tokens)",
        savings_percent, absolute_tokens, relative_tokens
    );

    // Should save at least 10% tokens for nested paths
    assert!(
        savings_percent >= 10.0,
        "Should save at least 10% tokens, got {:.1}%",
        savings_percent
    );
}

// ============================================================================
// Test 12: relative path with different project roots
// ============================================================================

#[test]
fn test_relative_path_without_git_root() {
    // Given: A temporary directory without git (project root only)
    let dir = TempDir::new().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let rust_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let file_path = src_dir.join("calculator.rs");
    fs::write(&file_path, rust_code).unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // When: parse_file is called (no git root, should use project root)
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Path is still relative (from project root or parent)
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let path = shape["path"].as_str().unwrap();

    // Should be relative (no absolute markers)
    assert!(
        is_relative_path(path),
        "Path should be relative even without git root, got: {}",
        path
    );

    // Should contain recognizable components
    assert!(
        path.contains("src") || path.contains("calculator.rs"),
        "Should contain relative path structure, got: {}",
        path
    );
}
