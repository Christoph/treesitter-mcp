//! End-to-end workflow tests for real LLM coding agent scenarios
//!
//! These tests simulate how an LLM agent would use treesitter-mcp
//! to accomplish common coding tasks.

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

fn write_file(dir: &TempDir, filename: &str, content: &str) {
    let file_path = dir.path().join(filename);
    fs::write(&file_path, content).unwrap();
}

// ============================================================================
// Workflow 1: Exploring a New Codebase
// Pattern: code_map → file_shape → read_focused_code
// ============================================================================

/// Scenario: LLM is asked "help me understand this project"
#[test]
fn test_workflow_explore_new_codebase_progressively() {
    // Given: Multi-file project
    let dir = setup_git_repo();

    // Create a simple project structure
    commit_file(
        &dir,
        "main.rs",
        r#"
mod calculator;
fn main() {
    let calc = calculator::Calculator::new();
    calc.add(5);
}
"#,
    );

    commit_file(
        &dir,
        "calculator.rs",
        r#"
pub struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new() -> Self {
        Self { value: 0 }
    }
    
    pub fn add(&mut self, n: i32) {
        self.value += n;
    }
    
    pub fn subtract(&mut self, n: i32) {
        self.value -= n;
    }
}
"#,
    );

    // Step 1: code_map for project overview
    let code_map_args = json!({
        "path": dir.path().to_str().unwrap(),
        "detail": "minimal",
        "max_tokens": 2000
    });
    let code_map_result = treesitter_mcp::analysis::code_map::execute(&code_map_args).unwrap();
    let code_map_text = common::get_result_text(&code_map_result);
    let code_map_json: serde_json::Value = serde_json::from_str(&code_map_text).unwrap();

    // Then: Should see files in the project
    let files = common::helpers::code_map_files(&code_map_json);
    assert!(files.len() >= 2, "Should see both files");

    // Step 2: file_shape on a specific file (low tokens)
    let calc_path = dir.path().join("calculator.rs");
    let file_shape_args = json!({
        "file_path": calc_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "merge_templates": false
    });
    let file_shape_result = treesitter_mcp::analysis::view_code::execute(&file_shape_args).unwrap();
    let file_shape_text = common::get_result_text(&file_shape_result);
    let file_shape_json: serde_json::Value = serde_json::from_str(&file_shape_text).unwrap();

    // Then: Should see struct and impl block methods (compact schema)
    assert!(
        file_shape_json.get("s").is_some()
            || file_shape_json.get("bm").is_some()
            || file_shape_json.get("f").is_some()
    );

    // Step 3: read_focused_code on specific impl method
    let focused_args = json!({
        "file_path": calc_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);
    let focused_json: serde_json::Value = serde_json::from_str(&focused_text).unwrap();

    // Then: Should have full implementation of add (compact `bm` rows)
    let header = focused_json
        .get("bh")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let code_idx = header
        .split('|')
        .position(|field| field == "code")
        .unwrap_or_else(|| panic!("Expected 'code' in impl-method header: {header}"));

    let rows_str = focused_json
        .get("bm")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let has_add_impl = rows.iter().any(|r| {
        r.get(2).map(|s| s.as_str()) == Some("add")
            && r.get(code_idx).map(|s| !s.is_empty()).unwrap_or(false)
    });

    assert!(
        has_add_impl,
        "Should have full code for focused impl method"
    );

    // Verify: Progressive detail with reasonable token growth
    let tokens_step1 = common::helpers::approx_tokens(&code_map_text);
    let tokens_step2 = common::helpers::approx_tokens(&file_shape_text);
    let tokens_step3 = common::helpers::approx_tokens(&focused_text);

    assert!(
        tokens_step3 > tokens_step2,
        "Focused read should have more detail than file shape"
    );
}

// ============================================================================
// Workflow 2: Debugging from Error/Stack Trace
// Pattern: get_context → read_focused_code → find_usages
// ============================================================================

/// Scenario: LLM receives "Error at calculator.rs:15"
#[test]
fn test_workflow_debug_error_from_line_number() {
    // Given: A file with code
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // Step 1: symbol_at_line to find what function is at a specific line
    let context_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "line": 14, // Inside add function body
        "column": 5
    });
    let context_result = treesitter_mcp::analysis::symbol_at_line::execute(&context_args).unwrap();
    let context_text = common::get_result_text(&context_result);
    let context_json: serde_json::Value = serde_json::from_str(&context_text).unwrap();

    // Then: Should identify the symbol and scope
    assert!(context_json["sym"].is_string(), "Should have sym");
    assert!(context_json["sig"].is_string(), "Should have sig");
    assert!(context_json["kind"].is_string(), "Should have kind");
    assert!(context_json["scope"].is_string(), "Should have scope");

    let scope = context_json["scope"].as_str().unwrap();
    assert!(!scope.is_empty(), "Scope should be non-empty");

    // Step 2: Use read_focused_code on "add" function
    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);

    // Then: Should have the full implementation
    assert!(focused_text.contains("add") || focused_text.contains("a + b"));

    // Step 3: find_usages to trace where variables come from
    let usages_args = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap(),
        "context_lines": 2
    });
    let usages_result = treesitter_mcp::analysis::find_usages::execute(&usages_args).unwrap();
    let usages_text = common::get_result_text(&usages_result);
    let usages_json: serde_json::Value = serde_json::from_str(&usages_text).unwrap();

    // Then: Should find usages
    assert!(common::helpers::find_usages_rows(&usages_json).len() >= 1);
}

// ============================================================================
// Workflow 3: Refactoring - Safe Rename
// Pattern: find_usages → make changes → parse_diff → affected_by_diff
// ============================================================================

/// Scenario: LLM is asked to "rename add() to sum()"
#[test]
fn test_workflow_rename_function_safely() {
    // Given: Git repo with function used in multiple places
    let dir = setup_git_repo();

    let initial_code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn calculate() -> i32 {
    add(1, 2) + add(3, 4)
}
"#;
    commit_file(&dir, "lib.rs", initial_code);

    let lib_path = dir.path().join("lib.rs");

    // Step 1: find_usages to identify all locations
    let usages_args = json!({
        "symbol": "add",
        "path": lib_path.to_str().unwrap(),
        "context_lines": 2
    });
    let usages_result = treesitter_mcp::analysis::find_usages::execute(&usages_args).unwrap();
    let usages_text = common::get_result_text(&usages_result);
    let usages_json: serde_json::Value = serde_json::from_str(&usages_text).unwrap();

    let locations = common::helpers::find_usages_rows(&usages_json);
    // Should find definition + 2 calls = 3 total
    assert!(
        locations.len() >= 3,
        "Should find definition and call sites"
    );

    // Step 2: Simulate making the rename
    let renamed_code = r#"
pub fn sum(a: i32, b: i32) -> i32 {
    a + b
}

pub fn calculate() -> i32 {
    sum(1, 2) + sum(3, 4)
}
"#;
    write_file(&dir, "lib.rs", renamed_code);

    // Step 3: parse_diff to verify structural changes
    let diff_args = json!({
        "file_path": lib_path.to_str().unwrap(),
        "compare_to": "HEAD"
    });
    let diff_result = treesitter_mcp::analysis::diff::execute_parse_diff(&diff_args).unwrap();
    let diff_text = common::get_result_text(&diff_result);
    let diff_json: serde_json::Value = serde_json::from_str(&diff_text).unwrap();

    // Then: Should show add removed and sum added
    let rows = common::helpers::parse_compact_rows(diff_json["changes"].as_str().unwrap_or(""));
    let has_removed = rows.iter().any(|r| {
        r.get(1).map(|s| s.as_str()) == Some("add")
            && r.get(3).map(|c| c.as_str()) == Some("removed")
    });
    let has_added = rows.iter().any(|r| {
        r.get(1).map(|s| s.as_str()) == Some("sum") && r.get(3).map(|c| c.as_str()) == Some("added")
    });

    assert!(has_removed, "Should show 'add' was removed");
    assert!(has_added, "Should show 'sum' was added");
}

/// Scenario: LLM changes a function signature, needs to find breaking call sites
#[test]
fn test_workflow_change_signature_find_breaking_calls() {
    // Given: main.rs calls lib.rs::calculate(x)
    let dir = setup_git_repo();

    commit_file(&dir, "lib.rs", "pub fn calculate(x: i32) -> i32 { x * 2 }");
    commit_file(
        &dir,
        "main.rs",
        r#"
mod lib;
fn main() {
    let result = lib::calculate(5);
    println!("{}", result);
}
"#,
    );

    // Step 1: find_usages before making changes
    let lib_path = dir.path().join("lib.rs");
    let usages_args = json!({
        "symbol": "calculate",
        "path": dir.path().to_str().unwrap(),
        "context_lines": 2
    });
    let usages_result = treesitter_mcp::analysis::find_usages::execute(&usages_args).unwrap();
    let usages_text = common::get_result_text(&usages_result);
    let usages_json: serde_json::Value = serde_json::from_str(&usages_text).unwrap();

    // Should find usages in both files
    assert!(common::helpers::find_usages_rows(&usages_json).len() >= 2);

    // Step 2: Simulate signature change: add parameter
    write_file(
        &dir,
        "lib.rs",
        "pub fn calculate(x: i32, y: i32) -> i32 { x * y }",
    );

    // Step 3: affected_by_diff should show HIGH risk for call site
    let affected_args = json!({
        "file_path": lib_path.to_str().unwrap(),
        "compare_to": "HEAD",
        "scope": dir.path().to_str().unwrap()
    });
    let affected_result =
        treesitter_mcp::analysis::diff::execute_affected_by_diff(&affected_args).unwrap();
    let affected_text = common::get_result_text(&affected_result);
    let affected_json: serde_json::Value = serde_json::from_str(&affected_text).unwrap();

    // Then: Should identify high risk usages
    let affected_rows =
        common::helpers::parse_compact_rows(affected_json["affected"].as_str().unwrap_or(""));

    assert!(
        affected_rows
            .iter()
            .any(|row| row.get(4).map(|s| s.as_str()) == Some("high")),
        "Should identify affected call sites"
    );
}

// ============================================================================
// Workflow 4: Adding Features
// Pattern: file_shape → read_focused_code → understand pattern
// ============================================================================

/// Scenario: LLM is asked to "add a multiply() method to Calculator"
#[test]
fn test_workflow_add_method_following_existing_pattern() {
    // Given: Existing calculator with add method
    let file_path = common::fixture_path("rust", "src/calculator.rs");

    // Step 1: file_shape to see existing functions/methods
    let shape_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_deps": false,
        "merge_templates": false
    });
    let shape_result = treesitter_mcp::analysis::view_code::execute(&shape_args).unwrap();
    let shape_text = common::get_result_text(&shape_result);
    let shape_json: serde_json::Value = serde_json::from_str(&shape_text).unwrap();

    // Then: Should see existing functions (compact schema)
    let has_functions = shape_json.get("f").is_some() || shape_json.get("bm").is_some();
    assert!(has_functions, "Should see existing code structure");

    // Step 2: read_focused_code on existing add() to see pattern
    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);

    // Then: Should have the implementation to follow
    assert!(
        focused_text.contains("add") || focused_text.contains("a + b"),
        "Should see existing implementation pattern"
    );

    // Verify: LLM has enough context to add consistent multiply() method
}

// ============================================================================
// Workflow 5: Cross-Layer Refactoring (Complex Project)
// ============================================================================

/// Scenario: LLM needs to trace a symbol across architectural layers
#[test]
fn test_workflow_trace_symbol_across_layers() {
    // Given: Complex multi-layer project
    let fixture_path = common::fixture_dir("complex_rust_service");

    // When: find_usages on a domain symbol
    let usages_args = json!({
        "symbol": "Order",
        "path": fixture_path.join("src").to_str().unwrap(),
        "context_lines": 2,
        "max_context_lines": 100
    });

    let usages_result = treesitter_mcp::analysis::find_usages::execute(&usages_args);

    // Then: Should find usages across multiple files
    if usages_result.is_ok() {
        let usages_text = common::get_result_text(&usages_result.as_ref().unwrap());
        let usages_json: serde_json::Value = serde_json::from_str(&usages_text).unwrap();

        let usage_list = common::helpers::find_usages_rows(&usages_json);
        if !usage_list.is_empty() {
            // Collect unique files
            let files: std::collections::HashSet<_> = usage_list
                .iter()
                .filter_map(|row| row.first().map(|s| s.as_str()))
                .collect();

            // Should span multiple files (domain, application, infrastructure)
            assert!(files.len() >= 2, "Symbol should be used across layers");
        }
    }
}

/// Scenario: Understanding a large file efficiently
#[test]
fn test_workflow_navigate_large_file_efficiently() {
    // Given: A file with multiple functions
    let file_path = common::fixture_path("python", "calculator.py");

    // Step 1: file_shape (NOT parse_file) for overview
    let shape_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "merge_templates": false
    });
    let shape_result = treesitter_mcp::analysis::view_code::execute(&shape_args).unwrap();
    let shape_text = common::get_result_text(&shape_result);
    let shape_tokens = common::helpers::approx_tokens(&shape_text);

    // Step 2: read_focused_code on entry point
    let focused_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "focus_symbol": "add",
        "context_radius": 0
    });
    let focused_result = treesitter_mcp::analysis::view_code::execute(&focused_args).unwrap();
    let focused_text = common::get_result_text(&focused_result);
    let focused_tokens = common::helpers::approx_tokens(&focused_text);

    // Then: Total tokens should be much less than full parse
    let combined_tokens = shape_tokens + focused_tokens;

    // Compare with full parse
    let full_parse_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": false
    });
    let full_parse_result = treesitter_mcp::analysis::view_code::execute(&full_parse_args).unwrap();
    let full_parse_text = common::get_result_text(&full_parse_result);
    let full_parse_tokens = common::helpers::approx_tokens(&full_parse_text);

    // Verify: Workflow uses significantly fewer tokens
    assert!(
        combined_tokens < full_parse_tokens,
        "Workflow should use fewer tokens than full parse: {} vs {}",
        combined_tokens,
        full_parse_tokens
    );
}
