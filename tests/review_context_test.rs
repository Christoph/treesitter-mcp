mod common;

use serde_json::json;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

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

#[test]
fn test_review_context_combines_changes_impact_tests_and_edit_context() {
    let dir = setup_git_repo();
    let src = dir.path().join("src");
    let tests_dir = dir.path().join("tests");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&tests_dir).unwrap();

    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(
        src.join("lib.rs"),
        r#"
pub fn calculate(x: i32) -> i32 {
    x * 2
}
"#,
    )
    .unwrap();
    fs::write(
        src.join("main.rs"),
        r#"
mod lib;

fn main() {
    let _ = lib::calculate(5);
}
"#,
    )
    .unwrap();
    fs::write(
        tests_dir.join("calculate_test.rs"),
        r#"
use fixture::calculate;

#[test]
fn test_calculate() {
    assert_eq!(calculate(5), 10);
}
"#,
    )
    .unwrap();

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
        src.join("lib.rs"),
        r#"
pub fn calculate(x: i64, y: i64) -> i64 {
    x * y
}
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::review_context::execute(&json!({
        "file_path": src.join("lib.rs").to_str().unwrap(),
        "compare_to": "HEAD",
        "scope": dir.path().to_str().unwrap(),
        "max_tokens": 6000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["p"], "src/lib.rs");
    assert_eq!(output["ch"], "type|name|line|change");
    assert_eq!(output["ah"], "symbol|change|file|line|risk");
    assert_eq!(output["th"], "symbol|test_file|test_fn|line|relevance");

    assert!(output["changes"].as_str().unwrap().contains("calculate"));
    assert!(output["affected"].as_str().unwrap().contains("main.rs"));
    assert!(output["tests"]
        .as_str()
        .unwrap()
        .contains("calculate_test.rs"));

    let ctx = output["ctx"].as_object().unwrap();
    let calculate = ctx.get("calculate").unwrap();
    assert!(calculate["target"].as_str().unwrap().contains("calculate"));
}
