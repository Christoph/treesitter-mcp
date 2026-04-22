mod common;

use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_format_diagnostics_accepts_lsp_diagnostics_with_owner_context() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("calculator.rs");
    fs::write(
        &file_path,
        "pub struct Calculator;\n\
\n\
impl Calculator {\n\
    pub fn add(&self, value: i32) -> i32 {\n\
        value + missing\n\
    }\n\
}\n",
    )
    .unwrap();

    let result = treesitter_mcp::analysis::format_diagnostics::execute(&json!({
        "diagnostics": [
            {
                "uri": format!("file://{}", file_path.display()),
                "range": {
                    "start": {
                        "line": 4,
                        "character": 16
                    }
                },
                "severity": 1,
                "message": "cannot find value `missing` in this scope",
                "source": "rustc",
                "code": "E0425"
            }
        ],
        "max_tokens": 2000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(
        output["h"],
        "severity|file|line|col|owner|source|code|message"
    );

    let rows = common::helpers::parse_compact_rows(output["d"].as_str().unwrap());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "error");
    assert!(rows[0][1].ends_with("calculator.rs"));
    assert_eq!(rows[0][2], "5");
    assert_eq!(rows[0][3], "17");
    assert_eq!(rows[0][4], "Calculator::add");
    assert_eq!(rows[0][5], "rustc");
    assert_eq!(rows[0][6], "E0425");
    assert!(rows[0][7].contains("missing"));
}

#[test]
fn test_format_diagnostics_accepts_compact_locations_and_orders_by_severity() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("workflow.ts");
    fs::write(
        &file_path,
        "export function buildSummary(value: string): string {\n\
  const unused = value;\n\
  return missingValue;\n\
}\n",
    )
    .unwrap();

    let result = treesitter_mcp::analysis::format_diagnostics::execute(&json!({
        "diagnostics": [
            {
                "file": file_path.to_str().unwrap(),
                "line": 2,
                "col": 9,
                "severity": 2,
                "message": "'unused' is declared but its value is never read.",
                "source": "typescript",
                "code": "6133"
            },
            {
                "file": file_path.to_str().unwrap(),
                "line": 3,
                "col": 10,
                "severity": 1,
                "message": "Cannot find name 'missingValue'.",
                "source": "typescript",
                "code": "2304"
            }
        ],
        "max_tokens": 2000
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();
    let rows = common::helpers::parse_compact_rows(output["d"].as_str().unwrap());

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0][0], "error");
    assert_eq!(rows[0][4], "buildSummary");
    assert_eq!(rows[1][0], "warning");
    assert_eq!(rows[1][4], "buildSummary");
}

#[test]
fn test_format_diagnostics_respects_token_budget() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("lib.ts");
    fs::write(&file_path, "export function run(): void {}\n").unwrap();

    let diagnostics: Vec<_> = (0..20)
        .map(|idx| {
            json!({
                "file": file_path.to_str().unwrap(),
                "line": 1,
                "col": 17,
                "severity": 2,
                "message": format!("diagnostic message number {idx} with extra context"),
                "source": "typescript",
                "code": "9999"
            })
        })
        .collect();

    let result = treesitter_mcp::analysis::format_diagnostics::execute(&json!({
        "diagnostics": diagnostics,
        "max_tokens": 90
    }))
    .unwrap();
    let text = common::get_result_text(&result);
    let output: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(output["@"]["t"], true);
    let rows = common::helpers::parse_compact_rows(output["d"].as_str().unwrap());
    assert!(rows.len() < 20);
}
