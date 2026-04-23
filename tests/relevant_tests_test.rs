mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn rows(value: &serde_json::Value, field: &str) -> Vec<Vec<String>> {
    common::helpers::parse_compact_rows(value[field].as_str().unwrap_or(""))
}

#[test]
fn test_relevant_tests_prioritizes_direct_test_calls() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();

    let src = dir.path().join("src");
    let tests_dir = dir.path().join("tests");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&tests_dir).unwrap();

    let lib_path = src.join("lib.rs");
    fs::write(
        &lib_path,
        r#"
pub fn calculate(x: i32) -> i32 {
    x * 2
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
    fs::write(
        tests_dir.join("other_test.rs"),
        r#"
#[test]
fn test_other() {
    assert_eq!(2 + 2, 4);
}
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::relevant_tests::execute(&json!({
        "file_path": lib_path.to_str().unwrap(),
        "symbol_name": "calculate"
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["sym"], "calculate");
    assert_eq!(output["h"], "test_file|test_fn|line|relevance");

    let test_rows = rows(&output, "tests");
    assert!(test_rows.iter().any(|row| {
        row[0].contains("calculate_test.rs") && row[1] == "test_calculate" && row[3] == "direct"
    }));
    assert!(!test_rows.iter().any(|row| row[0].contains("other_test.rs")));
}
