mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_format_references_accepts_lsp_locations() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("calculator.rs");
    fs::write(
        &file_path,
        "pub struct Calculator;\n\
\n\
impl Calculator {\n\
    pub fn add(&self, value: i32) -> i32 {\n\
        value + 1\n\
    }\n\
}\n\
\n\
pub fn run(calc: Calculator) -> i32 {\n\
    calc.add(41)\n\
}\n",
    )
    .unwrap();

    let arguments = json!({
        "symbol": "add",
        "references": [
            {
                "uri": format!("file://{}", file_path.display()),
                "range": {
                    "start": {
                        "line": 9,
                        "character": 9
                    }
                }
            }
        ],
        "context_lines": 1
    });

    let result = treesitter_mcp::analysis::format_references::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(json["sym"], "add");
    assert_eq!(json["h"], "file|line|col|type|context|scope|conf|owner");

    let rows = common::helpers::parse_compact_rows(json["u"].as_str().unwrap());
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert!(row[0].ends_with("calculator.rs"));
    assert_eq!(row[1], "10");
    assert_eq!(row[2], "10");
    assert_eq!(row[3], "call");
    assert!(row[4].contains("calc.add(41)"));
    assert!(row[5].contains("run"));
    assert_eq!(row[6], "high");
    assert_eq!(row[7], "calc");
}

#[test]
fn test_format_references_accepts_compact_locations() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("lib.ts");
    fs::write(
        &file_path,
        "export function parseInput(value: string): string {\n\
  return value.trim();\n\
}\n",
    )
    .unwrap();

    let arguments = json!({
        "symbol": "parseInput",
        "references": [
            {
                "file": file_path.to_str().unwrap(),
                "line": 1,
                "col": 17
            }
        ],
        "context_lines": 0
    });

    let result = treesitter_mcp::analysis::format_references::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = common::helpers::parse_compact_rows(json["u"].as_str().unwrap());

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][3], "definition");
    assert_eq!(
        rows[0][4],
        "export function parseInput(value: string): string {"
    );
    assert_eq!(rows[0][6], "high");
}
