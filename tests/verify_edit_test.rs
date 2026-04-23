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

#[test]
fn test_verify_edit_passes_when_only_target_symbol_changed() {
    let dir = setup_git_repo();
    commit_file(
        &dir,
        "lib.rs",
        r#"
pub fn calculate(x: i32) -> i32 { x * 2 }
pub fn helper(x: i32) -> i32 { x + 1 }
"#,
    );

    fs::write(
        dir.path().join("lib.rs"),
        r#"
pub fn calculate(x: i32) -> i32 { (x * 2) + 1 }
pub fn helper(x: i32) -> i32 { x + 1 }
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::verify_edit::execute(&json!({
        "file_path": dir.path().join("lib.rs").to_str().unwrap(),
        "compare_to": "HEAD",
        "target_symbol": "calculate"
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["ok"], true);
    let check_rows = rows(&output, "checks");
    assert!(check_rows
        .iter()
        .any(|row| row[0] == "target_changed" && row[1] == "ok"));
    assert!(check_rows
        .iter()
        .any(|row| row[0] == "unexpected_changes" && row[1] == "ok"));
}

#[test]
fn test_verify_edit_fails_when_unexpected_symbol_also_changed() {
    let dir = setup_git_repo();
    commit_file(
        &dir,
        "lib.rs",
        r#"
pub fn calculate(x: i32) -> i32 { x * 2 }
pub fn helper(x: i32) -> i32 { x + 1 }
"#,
    );

    fs::write(
        dir.path().join("lib.rs"),
        r#"
pub fn calculate(x: i32) -> i32 { (x * 2) + 1 }
pub fn helper(x: i64) -> i64 { x + 1 }
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::verify_edit::execute(&json!({
        "file_path": dir.path().join("lib.rs").to_str().unwrap(),
        "compare_to": "HEAD",
        "target_symbol": "calculate"
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["ok"], false);
    let check_rows = rows(&output, "checks");
    assert!(check_rows.iter().any(|row| {
        row[0] == "unexpected_changes" && row[1] == "fail" && row[2].contains("helper")
    }));
}
