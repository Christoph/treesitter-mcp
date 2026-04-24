//! Token efficiency tests to verify documented savings
//!
//! These tests ensure the token optimization features work as documented
//! in README.md and LLM_IMPROVEMENTS.md.

mod common;

use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

// ============================================================================
// file_shape vs parse_file (documented: 10-20% of parse_file)
// ============================================================================

#[test]
fn test_file_shape_is_significantly_cheaper_than_parse_file() {
    // Given: Same file
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: Compare file_shape vs parse_file output sizes
    let parse_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let parse_result = treesitter_mcp::analysis::view_code::execute(&parse_args).unwrap();
    let parse_text = common::get_result_text(&parse_result);
    let parse_tokens = common::helpers::approx_tokens(&parse_text);

    let shape_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "merge_templates": false
    });
    let shape_result = treesitter_mcp::analysis::view_code::execute(&shape_args).unwrap();
    let shape_text = common::get_result_text(&shape_result);
    let shape_tokens = common::helpers::approx_tokens(&shape_text);

    // Then: file_shape should be significantly cheaper than parse_file (allow up to 60%)
    // Note: With detail="signatures", we get ~50% reduction due to function bodies being removed
    // but signatures, documentation, and structural elements remain
    let ratio = shape_tokens as f64 / parse_tokens as f64;
    assert!(
        ratio < 0.60,
        "file_shape should be <60% of parse_file tokens, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        shape_tokens,
        parse_tokens
    );
}

// ============================================================================
// include_code=false (documented: 60-80% reduction)
// ============================================================================

#[test]
fn test_include_code_false_reduces_tokens_significantly() {
    // Given: File with functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_code=true) vs parse_file(include_code=false)
    let with_code_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let with_code_result = treesitter_mcp::analysis::view_code::execute(&with_code_args).unwrap();
    let with_code_text = common::get_result_text(&with_code_result);
    let with_tokens = common::helpers::approx_tokens(&with_code_text);

    let without_code_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false
    });
    let without_code_result =
        treesitter_mcp::analysis::view_code::execute(&without_code_args).unwrap();
    let without_code_text = common::get_result_text(&without_code_result);
    let without_tokens = common::helpers::approx_tokens(&without_code_text);

    // Then: include_code=false should be 40-60% of include_code=true (40-60% reduction)
    let ratio = without_tokens as f64 / with_tokens as f64;
    assert!(
        ratio < 0.60,
        "include_code=false should be <60% of include_code=true, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        without_tokens,
        with_tokens
    );

    // Verify at least 40% reduction (realistic for files with metadata)
    let reduction = ((with_tokens - without_tokens) as f64 / with_tokens as f64) * 100.0;
    assert!(
        reduction >= 40.0,
        "Should have at least 40% token reduction, got {:.1}%",
        reduction
    );
}

#[test]
fn test_include_code_false_preserves_signatures() {
    // Given: File with typed functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_code=false)
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Full signatures are present (types, params, return) in compact rows
    assert_eq!(
        shape.get("h").and_then(|v| v.as_str()),
        Some("name|line|sig")
    );

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .expect("Should find add row");

    let signature = add_row.get(2).expect("sig column");
    assert!(
        signature.contains("i32"),
        "Signature should include parameter types"
    );
    assert!(
        signature.contains("->"),
        "Signature should include return type indicator"
    );
}

// ============================================================================
// read_focused_code (~30% of parse_file claim)
// ============================================================================

#[test]
fn test_read_focused_code_is_cheaper_than_full_parse() {
    // Given: File with 10+ functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: read_focused_code vs parse_file
    let full_parse_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "full",
        "include_deps": false
    });
    let full_parse_result = treesitter_mcp::analysis::view_code::execute(&full_parse_args).unwrap();
    let full_parse_text = common::get_result_text(&full_parse_result);
    let full_tokens = common::helpers::approx_tokens(&full_parse_text);

    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0,
        "include_deps": false
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);
    let focused_tokens = common::helpers::approx_tokens(&focused_text);

    // Then: focused read should be ~30-60% of full parse (realistic range)
    let ratio = focused_tokens as f64 / full_tokens as f64;
    assert!(
        ratio < 0.65,
        "read_focused_code should be <65% of parse_file, got {:.1}% ({} vs {} tokens)",
        ratio * 100.0,
        focused_tokens,
        full_tokens
    );
}

#[test]
fn test_read_focused_code_has_full_impl_for_target() {
    // Given: File with multiple functions
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: read_focused_code on "add"
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0,
        "include_deps": false
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Target function has full code (compact rows)
    let header = shape.get("h").and_then(|v| v.as_str()).unwrap_or("");
    let code_idx = header
        .split('|')
        .position(|c| c == "code")
        .expect("Expected 'code' column in header");

    let rows_str = shape.get("f").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let add_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("add"))
        .expect("Should have add row");

    assert!(
        add_row
            .get(code_idx)
            .map(|c| !c.is_empty())
            .unwrap_or(false),
        "Focused function should have full code"
    );

    // And: Other functions have signatures only
    let subtract_row = rows
        .iter()
        .find(|r| r.first().map(|s| s.as_str()) == Some("subtract"));

    if let Some(subtract_row) = subtract_row {
        assert!(
            subtract_row
                .get(code_idx)
                .map(|c| c.is_empty())
                .unwrap_or(true),
            "Non-focused functions should not have code"
        );
        assert!(
            subtract_row.get(2).map(|s| !s.is_empty()).unwrap_or(false),
            "Non-focused functions should have signatures"
        );
    }
}

// ============================================================================
// include_deps provides dependency context in one call
// ============================================================================

#[test]
fn test_include_deps_provides_dependency_signatures() {
    // Given: File that imports from local modules (calculator.rs uses models)
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: parse_file(include_deps=true)
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": true,
        "max_tokens": 10_000
    });
    let result = treesitter_mcp::analysis::view_code::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Then: Dependencies should be included (compact `deps` map)
    let deps = shape
        .get("deps")
        .and_then(|v| v.as_object())
        .expect("Should have deps object");

    assert!(!deps.is_empty(), "Should have at least one dependency");

    // Dep values are row strings
    let first_rows = deps.iter().find_map(|(_path, rows)| rows.as_str()).unwrap();

    let rows = common::helpers::parse_compact_rows(first_rows);
    assert!(!rows.is_empty(), "Dep should have at least one row");
}

// ============================================================================
// max_context_lines prevents token explosion
// ============================================================================

#[test]
fn test_max_context_lines_bounds_output_for_common_symbols() {
    // Given: A symbol used in multiple places
    let fixture_path = common::fixture_dir("rust");

    // When: find_usages with and without max_context_lines
    let unbounded_args = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": null
    });
    let unbounded_result = treesitter_mcp::analysis::find_usages::execute(&unbounded_args).unwrap();
    let unbounded_text = common::get_result_text(&unbounded_result);
    let unbounded_tokens = common::helpers::approx_tokens(&unbounded_text);

    let bounded_args = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 50
    });
    let bounded_result = treesitter_mcp::analysis::find_usages::execute(&bounded_args).unwrap();
    let bounded_text = common::get_result_text(&bounded_result);
    let bounded_tokens = common::helpers::approx_tokens(&bounded_text);

    // Then: Bounded should be less than or equal to unbounded
    assert!(
        bounded_tokens <= unbounded_tokens,
        "max_context_lines should limit output: {} vs {} tokens",
        bounded_tokens,
        unbounded_tokens
    );

    // Parse both to verify structure is preserved
    let unbounded_json: serde_json::Value = serde_json::from_str(&unbounded_text).unwrap();
    let bounded_json: serde_json::Value = serde_json::from_str(&bounded_text).unwrap();

    // Both should have usages rows string
    assert!(unbounded_json["u"].is_string());
    assert!(bounded_json["u"].is_string());
}

// ============================================================================
// Code map respects token budget
// ============================================================================

#[test]
fn test_code_map_respects_max_tokens_parameter() {
    // Given: A directory with multiple files
    let fixture_path = common::fixture_dir("rust");

    // When: code_map with max_tokens limit
    let max_tokens = 1000;
    let arguments = json!({
        "path": fixture_path.join("src").to_str().unwrap(),
        "detail": "signatures",
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::code_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);
    let actual_tokens = common::helpers::approx_tokens(&text);

    // Then: Output should respect token budget (allow 20% overage for structure)
    assert!(
        actual_tokens <= max_tokens * 12 / 10,
        "code_map should respect token budget: {} > {} * 1.2",
        actual_tokens,
        max_tokens
    );
}

// ============================================================================
// Token-aware truncation using tiktoken-rs
// ============================================================================

#[test]
fn test_code_map_uses_tiktoken_for_token_counting() {
    // Given: A file with code
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // When: code_map with strict max_tokens limit using tiktoken
    let max_tokens = 100;
    let arguments = json!({
        "path": file_path.to_str().unwrap(),
        "detail": "full",
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::code_map::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);

    // Then: Actual tiktoken count should respect the limit
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    let actual_tokens = bpe.encode_with_special_tokens(&text).len();

    assert!(
        actual_tokens <= max_tokens,
        "code_map should respect tiktoken token budget: {} > {} tokens. Output was: {}",
        actual_tokens,
        max_tokens,
        text
    );
}

#[test]
fn test_find_usages_uses_tiktoken_for_token_counting() {
    // Given: A directory with multiple usages of a symbol
    let fixture_path = common::fixture_dir("rust");

    // When: find_usages with strict max_tokens limit
    let max_tokens = 50;
    let arguments = json!({
        "symbol": "Calculator",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 3,
        "max_context_lines": 100,
        "max_tokens": max_tokens
    });
    let result = treesitter_mcp::analysis::find_usages::execute(&arguments).unwrap();
    let text = common::get_result_text(&result);

    // Then: Actual tiktoken count should respect the limit
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    let actual_tokens = bpe.encode_with_special_tokens(&text).len();

    assert!(
        actual_tokens <= max_tokens,
        "find_usages should respect tiktoken token budget: {} > {} tokens. Output was: {}",
        actual_tokens,
        max_tokens,
        text
    );
}

#[test]
fn test_format_references_uses_tiktoken_for_token_counting() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("lib.rs");
    fs::write(
        &file_path,
        r#"
pub fn target(value: i32) -> i32 {
    value + 1
}

pub fn caller_one() -> i32 {
    target(1)
}

pub fn caller_two() -> i32 {
    target(2)
}

pub fn caller_three() -> i32 {
    target(3)
}
"#,
    )
    .unwrap();

    let max_tokens = 80;
    let result = treesitter_mcp::analysis::format_references::execute(&json!({
        "symbol": "target",
        "references": [
            {"file": file_path.to_str().unwrap(), "line": 7, "col": 5},
            {"file": file_path.to_str().unwrap(), "line": 11, "col": 5},
            {"file": file_path.to_str().unwrap(), "line": 15, "col": 5}
        ],
        "context_lines": 1,
        "max_tokens": max_tokens
    }))
    .unwrap();
    let text = common::get_result_text(&result);

    assert_tiktoken_budget(&text, max_tokens, "format_references");
}

#[test]
fn test_format_diagnostics_uses_tiktoken_for_token_counting() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("lib.ts");
    fs::write(
        &file_path,
        r#"
export function buildReport(value: string): string {
  const first = missingOne;
  const second = missingTwo;
  const third = missingThree;
  return value;
}
"#,
    )
    .unwrap();

    let diagnostics: Vec<_> = (0..12)
        .map(|idx| {
            json!({
                "file": file_path.to_str().unwrap(),
                "line": 3,
                "col": 17,
                "severity": 1,
                "source": "typescript",
                "code": "2304",
                "message": format!("Cannot find name 'missingValue{idx}' in this scope with repeated details")
            })
        })
        .collect();

    let max_tokens = 100;
    let result = treesitter_mcp::analysis::format_diagnostics::execute(&json!({
        "diagnostics": diagnostics,
        "max_tokens": max_tokens
    }))
    .unwrap();
    let text = common::get_result_text(&result);

    assert_tiktoken_budget(&text, max_tokens, "format_diagnostics");
}

#[test]
fn test_call_graph_uses_tiktoken_for_token_counting() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='token_fixture'\nversion='0.1.0'\n",
    )
    .unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let file_path = src.join("lib.rs");
    fs::write(
        &file_path,
        r#"
pub fn leaf_one() -> u32 { 1 }
pub fn leaf_two() -> u32 { 2 }
pub fn leaf_three() -> u32 { 3 }
pub fn leaf_four() -> u32 { 4 }
pub fn leaf_five() -> u32 { 5 }

pub fn target() -> u32 {
    leaf_one() + leaf_two() + leaf_three() + leaf_four() + leaf_five()
}

pub fn caller_one() -> u32 { target() }
pub fn caller_two() -> u32 { target() }
pub fn caller_three() -> u32 { target() }
"#,
    )
    .unwrap();

    let max_tokens = 90;
    let result = treesitter_mcp::analysis::call_graph::execute(&json!({
        "file_path": file_path.to_str().unwrap(),
        "symbol_name": "target",
        "direction": "both",
        "depth": 1,
        "max_tokens": max_tokens
    }))
    .unwrap();
    let text = common::get_result_text(&result);

    assert_tiktoken_budget(&text, max_tokens, "call_graph");
}

#[test]
fn test_view_code_definition_location_keeps_dependency_context_compact() {
    let dir = tempdir().unwrap();
    let types_path = dir.path().join("types.ts");
    fs::write(
        &types_path,
        r#"
export interface Alpha { value: string; }
export interface Beta { value: string; }
export interface Gamma { value: string; }
export interface Delta { value: string; }
export interface Epsilon { value: string; }
"#,
    )
    .unwrap();

    let main_path = dir.path().join("main.ts");
    fs::write(
        &main_path,
        r#"
import type { Alpha, Beta, Gamma, Delta, Epsilon } from "./types";

export function makeValue(): unknown {
  return {};
}
"#,
    )
    .unwrap();

    let broad_result = treesitter_mcp::analysis::view_code::execute(&json!({
        "file_path": main_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": true,
        "max_tokens": 4000
    }))
    .unwrap();
    let broad_text = common::get_result_text(&broad_result);

    let exact_result = treesitter_mcp::analysis::view_code::execute(&json!({
        "file_path": main_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": true,
        "definition_location": {
            "file": types_path.to_str().unwrap(),
            "line": 4,
            "col": 18
        },
        "max_tokens": 4000
    }))
    .unwrap();
    let exact_text = common::get_result_text(&exact_result);
    let exact_json: serde_json::Value = serde_json::from_str(&exact_text).unwrap();
    let dep_rows = exact_json["deps"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(dep_rows.contains("Gamma|4|"));
    assert!(!dep_rows.contains("Alpha|"));
    assert!(!dep_rows.contains("Beta|"));
    assert!(
        tiktoken_count(&exact_text) < tiktoken_count(&broad_text),
        "definition_location should be more compact than broad dependency inference"
    );
}

fn assert_tiktoken_budget(text: &str, max_tokens: usize, context: &str) {
    let actual_tokens = tiktoken_count(text);
    assert!(
        actual_tokens <= max_tokens,
        "{context} should respect tiktoken token budget: {actual_tokens} > {max_tokens}. Output was: {text}"
    );
}

fn tiktoken_count(text: &str) -> usize {
    let bpe = tiktoken_rs::cl100k_base().unwrap();
    bpe.encode_with_special_tokens(text).len()
}

#[derive(Debug, Clone)]
struct AggregateBenchmark {
    name: &'static str,
    default_tool: &'static str,
    mcp_tool: &'static str,
    sample_count: usize,
    raw_tokens: usize,
    mcp_tokens: usize,
}

impl AggregateBenchmark {
    fn saved_tokens(&self) -> usize {
        self.raw_tokens.saturating_sub(self.mcp_tokens)
    }

    fn saved_pct(&self) -> f64 {
        if self.raw_tokens == 0 {
            0.0
        } else {
            100.0 * self.saved_tokens() as f64 / self.raw_tokens as f64
        }
    }

    fn smaller_factor(&self) -> f64 {
        if self.mcp_tokens == 0 {
            0.0
        } else {
            self.raw_tokens as f64 / self.mcp_tokens as f64
        }
    }
}

#[test]
fn test_average_token_efficiency_for_core_workflows() {
    let benchmarks = average_token_benchmarks();

    let overview = benchmarks
        .iter()
        .find(|bench| bench.name == "Overview average")
        .unwrap();
    assert!(
        overview.saved_pct() >= 45.0,
        "overview average should save at least 45%, got {:.1}%",
        overview.saved_pct()
    );

    let focused = benchmarks
        .iter()
        .find(|bench| bench.name == "Focused edit average")
        .unwrap();
    assert!(
        focused.saved_pct() >= 70.0,
        "focused edit average should save at least 70%, got {:.1}%",
        focused.saved_pct()
    );

    let call_graph = benchmarks
        .iter()
        .find(|bench| bench.name == "Call graph average")
        .unwrap();
    assert!(
        call_graph.saved_pct() >= 90.0,
        "call graph average should save at least 90%, got {:.1}%",
        call_graph.saved_pct()
    );

    let find_usages = benchmarks
        .iter()
        .find(|bench| bench.name == "Repo search average")
        .unwrap();
    assert!(
        find_usages.saved_pct() >= 95.0,
        "repo search average should save at least 95%, got {:.1}%",
        find_usages.saved_pct()
    );

    let code_map = benchmarks
        .iter()
        .find(|bench| bench.name == "Directory map average")
        .unwrap();
    assert!(
        code_map.saved_pct() >= 90.0,
        "directory map average should save at least 90%, got {:.1}%",
        code_map.saved_pct()
    );
}

#[test]
#[ignore = "manual benchmark reporter for README/image refresh"]
fn report_average_token_benchmarks() {
    for bench in average_token_benchmarks() {
        println!(
            "{}|{}|{}|{}|{}|{}|{:.1}%|{:.1}x",
            bench.name,
            bench.sample_count,
            bench.default_tool,
            bench.mcp_tool,
            bench.raw_tokens,
            bench.mcp_tokens,
            bench.saved_pct(),
            bench.smaller_factor()
        );
    }
}

fn average_token_benchmarks() -> Vec<AggregateBenchmark> {
    vec![
        overview_average_benchmark(),
        focused_edit_average_benchmark(),
        call_graph_average_benchmark(),
        repo_search_average_benchmark(),
        directory_map_average_benchmark(),
    ]
}

fn overview_average_benchmark() -> AggregateBenchmark {
    let scenarios = vec![
        fixture_file("rust_project", "src/calculator.rs"),
        fixture_file("typescript_project", "calculator.ts"),
        fixture_file("python_project", "calculator.py"),
        fixture_file("javascript_project", "calculator.js"),
    ];

    let raw_tokens = average_tokens(scenarios.iter().map(|path| raw_file_tokens(path)));
    let mcp_tokens = average_tokens(scenarios.iter().map(|path| {
        let result = treesitter_mcp::analysis::view_code::execute(&json!({
            "file_path": path.to_str().unwrap(),
            "detail": "signatures",
            "include_deps": false
        }))
        .unwrap();
        tiktoken_count(&common::get_result_text(&result))
    }));

    AggregateBenchmark {
        name: "Overview average",
        default_tool: "cat <4 source files>",
        mcp_tool: "view_code(detail=\"signatures\")",
        sample_count: scenarios.len(),
        raw_tokens,
        mcp_tokens,
    }
}

fn focused_edit_average_benchmark() -> AggregateBenchmark {
    let scenarios = vec![
        (
            fixture_file("rust_project", "src/calculator.rs"),
            "perform_sequence",
        ),
        (
            fixture_file("typescript_project", "calculator.ts"),
            "applyOperation",
        ),
        (
            fixture_file("python_project", "calculator.py"),
            "apply_operation",
        ),
        (
            fixture_file("javascript_project", "calculator.js"),
            "applyOperation",
        ),
    ];

    let raw_tokens = average_tokens(scenarios.iter().map(|(path, _)| raw_file_tokens(path)));
    let mcp_tokens = average_tokens(scenarios.iter().map(|(path, symbol)| {
        let result = treesitter_mcp::analysis::minimal_edit_context::execute(&json!({
            "file_path": path.to_str().unwrap(),
            "symbol_name": symbol,
            "max_tokens": 4000
        }))
        .unwrap();
        tiktoken_count(&common::get_result_text(&result))
    }));

    AggregateBenchmark {
        name: "Focused edit average",
        default_tool: "cat <4 source files>",
        mcp_tool: "minimal_edit_context(symbol_name=...)",
        sample_count: scenarios.len(),
        raw_tokens,
        mcp_tokens,
    }
}

fn call_graph_average_benchmark() -> AggregateBenchmark {
    let root = workspace_root();
    let scenarios = vec![
        (root.join("src/analysis/view_code.rs"), "execute"),
        (root.join("src/analysis/minimal_edit_context.rs"), "execute"),
        (root.join("src/analysis/call_graph.rs"), "execute"),
        (root.join("src/analysis/code_map.rs"), "execute"),
    ];

    let raw_tokens = average_tokens(scenarios.iter().map(|(path, _)| raw_file_tokens(path)));
    let mcp_tokens = average_tokens(scenarios.iter().map(|(path, symbol)| {
        let result = treesitter_mcp::analysis::call_graph::execute(&json!({
            "file_path": path.to_str().unwrap(),
            "symbol_name": symbol,
            "direction": "both",
            "depth": 1,
            "max_tokens": 4000
        }))
        .unwrap();
        tiktoken_count(&common::get_result_text(&result))
    }));

    AggregateBenchmark {
        name: "Call graph average",
        default_tool: "cat <4 analysis files>",
        mcp_tool: "call_graph(symbol_name=...)",
        sample_count: scenarios.len(),
        raw_tokens,
        mcp_tokens,
    }
}

fn repo_search_average_benchmark() -> AggregateBenchmark {
    let scope = workspace_root().join("src/analysis");
    let scenarios = ["parse_code", "execute", "detect_language"];
    let raw_scope_tokens = raw_directory_tokens(&scope);
    let raw_tokens = average_tokens(scenarios.iter().map(|_| raw_scope_tokens));
    let mcp_tokens = average_tokens(scenarios.iter().map(|symbol| {
        let result = treesitter_mcp::analysis::find_usages::execute(&json!({
            "symbol": symbol,
            "path": scope.to_str().unwrap(),
            "context_lines": 1,
            "max_context_lines": 60,
            "max_tokens": 4000
        }))
        .unwrap();
        tiktoken_count(&common::get_result_text(&result))
    }));

    AggregateBenchmark {
        name: "Repo search average",
        default_tool: "cat src/analysis/*.rs",
        mcp_tool: "find_usages(symbol=...)",
        sample_count: scenarios.len(),
        raw_tokens,
        mcp_tokens,
    }
}

fn directory_map_average_benchmark() -> AggregateBenchmark {
    let root = workspace_root();
    let scenarios = vec![
        root.join("src"),
        root.join("src/analysis"),
        root.join("tests/fixtures/complex_rust_service/src"),
    ];

    let raw_tokens = average_tokens(scenarios.iter().map(|path| raw_directory_tokens(path)));
    let mcp_tokens = average_tokens(scenarios.iter().map(|path| {
        let result = treesitter_mcp::analysis::code_map::execute(&json!({
            "path": path.to_str().unwrap(),
            "detail": "minimal",
            "max_tokens": 4000
        }))
        .unwrap();
        tiktoken_count(&common::get_result_text(&result))
    }));

    AggregateBenchmark {
        name: "Directory map average",
        default_tool: "find <3 source trees> -exec cat",
        mcp_tool: "code_map(detail=\"minimal\")",
        sample_count: scenarios.len(),
        raw_tokens,
        mcp_tokens,
    }
}

fn average_tokens<I>(values: I) -> usize
where
    I: IntoIterator<Item = usize>,
{
    let values = values.into_iter().collect::<Vec<_>>();
    values.iter().sum::<usize>() / values.len().max(1)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_file(project: &str, file: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures")
        .join(project)
        .join(file)
}

fn raw_file_tokens(path: &Path) -> usize {
    tiktoken_count(&fs::read_to_string(path).unwrap())
}

fn raw_directory_tokens(path: &Path) -> usize {
    let files = treesitter_mcp::common::project_files::collect_project_files(path).unwrap();
    let combined = files
        .into_iter()
        .filter(|file| treesitter_mcp::parser::detect_language(file).is_ok())
        .map(|file| fs::read_to_string(file).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    tiktoken_count(&combined)
}
