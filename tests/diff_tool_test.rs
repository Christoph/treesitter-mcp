mod common;

use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn rows(value: &serde_json::Value, field: &str) -> Vec<Vec<String>> {
    common::helpers::parse_compact_rows(value[field].as_str().unwrap_or(""))
}

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
// parse_diff Tests (compact schema)
// ============================================================================

#[test]
fn test_parse_diff_no_changes() {
    let dir = setup_git_repo();
    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(analysis["p"].as_str().unwrap().contains("lib.rs"));
    assert_eq!(analysis["h"], "type|name|line|change");
    assert_eq!(analysis["changes"].as_str().unwrap_or(""), "");
}

#[test]
fn test_parse_diff_function_added() {
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    fs::write(
        dir.path().join("lib.rs"),
        "fn add(a: i32, b: i32) -> i32 { a + b }\nfn subtract(a: i32, b: i32) -> i32 { a - b }\n",
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

    let changes = rows(&analysis, "changes");
    assert!(
        changes
            .iter()
            .any(|r| r.get(1).map(|s| s.as_str()) == Some("subtract")
                && r.get(3).map(|s| s.as_str()) == Some("added")),
        "Should include subtract as added"
    );
}

#[test]
fn test_parse_diff_go_function_added() {
    let dir = setup_git_repo();

    commit_file(
        &dir,
        "lib.go",
        "package main\n\nfunc add(a int, b int) int { return a + b }\n",
    );

    fs::write(
        dir.path().join("lib.go"),
        "package main\n\nfunc add(a int, b int) int { return a + b }\nfunc subtract(a int, b int) int { return a - b }\n",
    )
    .unwrap();

    let file_path = dir.path().join("lib.go");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    let changes = rows(&analysis, "changes");
    assert!(
        changes
            .iter()
            .any(|r| r.get(1).map(|s| s.as_str()) == Some("subtract")
                && r.get(3).map(|s| s.as_str()) == Some("added")),
        "Should include subtract as added"
    );
}

#[test]
fn test_parse_diff_signature_changed() {
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

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

    let changes = rows(&analysis, "changes");
    let sig_change = changes
        .iter()
        .find(|r| r.get(1).map(|s| s.as_str()) == Some("add"));
    assert!(sig_change.is_some());

    let sig_change = sig_change.unwrap();
    let change_text = sig_change.get(3).map(|s| s.as_str()).unwrap_or("");
    assert!(change_text.contains("sig_changed"));
    assert!(change_text.contains("i64"));
}

#[test]
fn test_parse_diff_body_only_change() {
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

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

    let changes = rows(&analysis, "changes");
    assert!(
        changes
            .iter()
            .any(|r| r.get(3).map(|s| s.as_str()) == Some("body_changed")),
        "Should include body_changed row"
    );
}

#[test]
fn test_parse_diff_function_removed() {
    let dir = setup_git_repo();

    commit_file(
        &dir,
        "lib.rs",
        "fn add(a: i32, b: i32) -> i32 { a + b }\nfn subtract(a: i32, b: i32) -> i32 { a - b }\n",
    );

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

    let changes = rows(&analysis, "changes");
    assert!(
        changes
            .iter()
            .any(|r| r.get(1).map(|s| s.as_str()) == Some("subtract")
                && r.get(3).map(|s| s.as_str()) == Some("removed")),
        "Should include subtract as removed"
    );
}

#[test]
fn test_parse_diff_compare_to_older_commit() {
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "fn v1() {}");
    commit_file(&dir, "lib.rs", "fn v1() {}\nfn v2() {}");

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD~1"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let analysis: serde_json::Value = serde_json::from_str(&text).unwrap();

    let changes = rows(&analysis, "changes");
    assert!(
        changes
            .iter()
            .any(|r| r.get(1).map(|s| s.as_str()) == Some("v2")
                && r.get(3).map(|s| s.as_str()) == Some("added")),
        "Should show v2 as added when comparing to older commit"
    );
}

// ============================================================================
// affected_by_diff Tests (compact schema)
// ============================================================================

#[test]
fn test_affected_by_diff_finds_call_sites() {
    let dir = setup_git_repo();

    let lib_content = "pub fn calculate(x: i32) -> i32 { x * 2 }";
    let main_content = r#"
mod lib;
fn main() {
    let result = lib::calculate(5);
    println!(\"{}\", result);
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

    assert_eq!(affected["h"], "symbol|change|file|line|risk");

    let rows = rows(&affected, "affected");
    assert!(!rows.is_empty());

    // Expect at least one high-risk call site in main.rs
    assert!(rows.iter().any(|r| {
        r.get(2).map(|f| f.contains("main.rs")).unwrap_or(false)
            && r.get(4).map(|risk| risk.as_str()) == Some("high")
    }));
}

#[test]
fn test_affected_by_diff_no_structural_changes() {
    let dir = setup_git_repo();
    commit_file(&dir, "lib.rs", "fn add(a: i32, b: i32) -> i32 { a + b }");

    let file_path = dir.path().join("lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_affected_by_diff(&arguments);
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let affected: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(affected["affected"].as_str().unwrap_or(""), "");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_diff_not_git_repo() {
    let dir = TempDir::new().unwrap();
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

    let file_path = dir.path().join("new.rs");
    fs::write(&file_path, "fn test() {}").unwrap();

    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });

    let result = treesitter_mcp::analysis::diff::execute_parse_diff(&arguments);
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
