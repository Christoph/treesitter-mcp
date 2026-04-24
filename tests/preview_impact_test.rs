mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

fn rows(value: &serde_json::Value, field: &str) -> Vec<Vec<String>> {
    common::helpers::parse_compact_rows(value[field].as_str().unwrap_or(""))
}

#[test]
fn test_preview_impact_reports_signature_blast_radius_before_edit() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();

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
        src.join("main.rs"),
        r#"
mod lib;

fn main() {
    let result = lib::calculate(5);
    println!("{}", result);
}
"#,
    )
    .unwrap();

    let result = treesitter_mcp::analysis::diff::execute_preview_impact(&json!({
        "file_path": lib_path.to_str().unwrap(),
        "symbol_name": "calculate",
        "new_signature": "pub fn calculate(x: i64, y: i64) -> i64",
        "scope": dir.path().to_str().unwrap()
    }))
    .unwrap();

    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["sym"], "calculate");
    assert_eq!(output["h"], "symbol|change|file|line|risk");
    assert_eq!(output["dh"], "kind|name|from|to");
    assert!(output["before"].as_str().unwrap().contains("i32"));
    assert!(output["after"].as_str().unwrap().contains("i64"));

    let detail_rows = rows(&output, "d");
    assert!(detail_rows.iter().any(|row| row[0] == "parameter_count"));
    assert!(detail_rows.iter().any(|row| row[0] == "return_type"));

    let affected_rows = rows(&output, "affected");
    assert!(affected_rows
        .iter()
        .any(|row| row[2].contains("main.rs") && row[4] == "high"));
}
