//! Structural verification for an edit.

use std::io;

use serde_json::{json, Value};

use crate::analysis::diff;
use crate::common::format;
use crate::mcp_types::{CallToolResult, CallToolResultExt};

const CHECK_HEADER: &str = "check|status|detail";

struct CheckRow {
    check: &'static str,
    status: &'static str,
    detail: String,
}

/// Verify that an edit touched the intended symbol and avoided extra changes.
pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;
    let compare_to = arguments["compare_to"]
        .as_str()
        .unwrap_or("HEAD")
        .to_string();
    let target_symbol = arguments["target_symbol"].as_str();

    let analysis = diff::analyze_diff(file_path, compare_to.clone())?;
    let mut checks = Vec::new();

    let changed_symbols = analysis
        .structural_changes
        .iter()
        .map(|change| change.name.clone())
        .collect::<Vec<_>>();
    checks.push(CheckRow {
        check: "parse_diff",
        status: "ok",
        detail: if changed_symbols.is_empty() {
            "No structural changes detected".to_string()
        } else {
            format!(
                "{} structural change(s): {}",
                changed_symbols.len(),
                changed_symbols.join(", ")
            )
        },
    });

    if let Some(target_symbol) = target_symbol {
        let target_changed = analysis
            .structural_changes
            .iter()
            .any(|change| change.name == target_symbol);
        checks.push(CheckRow {
            check: "target_changed",
            status: if target_changed { "ok" } else { "fail" },
            detail: if target_changed {
                format!("Target symbol '{target_symbol}' changed")
            } else {
                format!("Target symbol '{target_symbol}' was not changed")
            },
        });

        let unexpected = analysis
            .structural_changes
            .iter()
            .filter(|change| change.name != target_symbol)
            .map(|change| change.name.clone())
            .collect::<Vec<_>>();
        checks.push(CheckRow {
            check: "unexpected_changes",
            status: if unexpected.is_empty() { "ok" } else { "fail" },
            detail: if unexpected.is_empty() {
                "No unexpected symbol changes".to_string()
            } else {
                format!("Unexpected symbol changes: {}", unexpected.join(", "))
            },
        });
    }

    let ok = checks.iter().all(|check| check.status != "fail");
    let rows = checks
        .iter()
        .map(|check| format::format_row(&[check.check, check.status, &check.detail]))
        .collect::<Vec<_>>()
        .join("\n");

    let result = json!({
        "p": analysis.file_path,
        "cmp": compare_to,
        "ok": ok,
        "h": CHECK_HEADER,
        "checks": rows,
    });
    let result_json = serde_json::to_string(&result).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize verify_edit result: {e}"),
        )
    })?;

    Ok(CallToolResult::success(result_json))
}
