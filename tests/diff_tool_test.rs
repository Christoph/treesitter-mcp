mod common;

use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

// ============================================================================
// Setup Helpers
// ============================================================================

fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Configure git user (required for commits)
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

fn commit_file(dir: &TempDir, filename: &str, content: &str) {
    let file_path = dir.path().join(filename);
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
// parse_diff Tests
// ============================================================================

#[test]
fn test_parse_diff_no_changes() {
    let dir = setup_git_repo();
    let content = "fn add(a: i32, b: i32) -> i32 { a + b }";
    commit_file(&dir, "lib.rs", content);

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(analysis["no_structural_change"], true);
    assert_eq!(analysis["structural_changes"].as_array().unwrap().len(), 0);
}

#[test]
fn test_parse_diff_function_added() {
    let dir = setup_git_repo();

    // Initial commit
    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    // Add a new function (uncommitted)
    let new_content = r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(a: i32, b: i32) -> i32 { a - b }
"#;
    fs::write(dir.path().join("lib.rs"), new_content).unwrap();

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(analysis["no_structural_change"], false);
    assert_eq!(analysis["summary"]["added"], 1);

    let changes = analysis["structural_changes"].as_array().unwrap();
    let added = changes
        .iter()
        .find(|c| c["change_type"] == "added")
        .unwrap();
    assert_eq!(added["name"], "subtract");
}

#[test]
fn test_parse_diff_signature_changed() {
    let dir = setup_git_repo();

    // Initial commit with i32
    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    // Change to i64 (uncommitted)
    fs::write(
        dir.path().join("lib.rs"),
        "fn add(a: i64, b: i64) -> i64 { a + b }",
    )
    .unwrap();

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(analysis["summary"]["modified"], 1);

    let changes = analysis["structural_changes"].as_array().unwrap();
    let modified = changes
        .iter()
        .find(|c| c["change_type"] == "signature_changed")
        .unwrap();
    assert_eq!(modified["name"], "add");
    assert!(modified["before"].as_str().unwrap().contains("i32"));
    assert!(modified["after"].as_str().unwrap().contains("i64"));
}

#[test]
fn test_parse_diff_body_only_change() {
    let dir = setup_git_repo();

    // Initial commit
    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    // Change body only (uncommitted)
    fs::write(
        dir.path().join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { let sum = a + b; sum }",
    )
    .unwrap();

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    let changes = analysis["structural_changes"].as_array().unwrap();
    let body_change = changes.iter().find(|c| c["change_type"] == "body_changed");
    assert!(body_change.is_some());
}

#[test]
fn test_parse_diff_function_removed() {
    let dir = setup_git_repo();

    // Initial commit with two functions
    commit_file(
        &dir,
        "lib.rs",
        r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn subtract(a: i32, b: i32) -> i32 { a - b }
"#,
    );

    // Remove subtract (uncommitted)
    fs::write(
        dir.path().join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }",
    )
    .unwrap();

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(analysis["summary"]["removed"], 1);

    let changes = analysis["structural_changes"].as_array().unwrap();
    let removed = changes
        .iter()
        .find(|c| c["change_type"] == "removed")
        .unwrap();
    assert_eq!(removed["name"], "subtract");
}

#[test]
fn test_parse_diff_compare_to_older_commit() {
    let dir = setup_git_repo();

    // First commit
    commit_file(&dir, "lib.rs", "fn v1() {}");

    // Second commit
    commit_file(&dir, "lib.rs", "fn v1() {}\nfn v2() {}");

    // Compare current to HEAD~1
    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD~1"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(analysis["summary"]["added"], 1);
}

// ============================================================================
// affected_by_diff Tests
// ============================================================================

#[test]
fn test_affected_by_diff_finds_call_sites() {
    let dir = setup_git_repo();

    // Create initial files
    let lib_content = "pub fn calculate(x: i32) -> i32 { x * 2 }";
    let main_content = r#"
mod lib;
fn main() {
    let result = lib::calculate(5);
    println!("{}", result);
}
"#;

    fs::write(dir.path().join("lib.rs"), lib_content).unwrap();
    fs::write(dir.path().join("main.rs"), main_content).unwrap();

    Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Change signature (uncommitted)
    fs::write(
        dir.path().join("lib.rs"),
        "pub fn calculate(x: i64, y: i64) -> i64 { x * y }",
    )
    .unwrap();

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD",
        "scope": dir.path().to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::diff::execute_affected_by_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let affected: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(affected["summary"]["total_usages"].as_u64().unwrap() >= 1);
    assert!(affected["summary"]["high_risk"].as_u64().unwrap() >= 1);
}

#[test]
fn test_affected_by_diff_no_structural_changes() {
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    // No changes
    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_affected_by_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let affected: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(affected["affected_changes"].as_array().unwrap().len(), 0);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_diff_not_git_repo() {
    let dir = TempDir::new().unwrap(); // Not a git repo
    let file_path = dir.path().join("test.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_err());
}

#[test]
fn test_parse_diff_file_not_in_git() {
    let dir = setup_git_repo();

    // Create file but don't commit it
    let file_path = dir.path().join("new.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    // Should fail because file doesn't exist in HEAD
    assert!(result.is_err());
}

#[test]
fn test_parse_diff_invalid_revision() {
    let dir = setup_git_repo();
    commit_file(&dir, "lib.rs", "fn test() {}");

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "nonexistent-branch"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_err());
}
