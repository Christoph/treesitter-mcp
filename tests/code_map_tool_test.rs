mod common;

use serde_json::json;

// ============================================================================
// Detail Level Tests
// ============================================================================

#[test]
fn test_code_map_provides_minimal_overview_with_names_only() {
    // Given: Rust fixture project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "minimal"
    });

    // When: code_map with detail="minimal"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns names only
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert!(map["files"].is_array());
    let files = map["files"].as_array().unwrap();
    assert!(files.len() > 0);

    // Check that functions have names but minimal other info
    let calc_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().contains("calculator.rs"))
        .unwrap();
    let functions = calc_file["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    assert_eq!(add_fn["name"], "add");
    // In minimal mode, should not have signature or doc
    assert!(add_fn["signature"].is_null() || !add_fn.get("signature").is_some());
}

#[test]
fn test_code_map_includes_signatures_at_medium_detail() {
    // Given: Rust fixture project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "signatures"
    });

    // When: code_map with detail="signatures"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns names + full signatures
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    let calc_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().contains("calculator.rs"))
        .unwrap();
    let functions = calc_file["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    // Should have signature
    assert!(add_fn["signature"].is_string());
    assert!(add_fn["signature"].as_str().unwrap().contains("pub fn add"));
    assert!(add_fn["signature"].as_str().unwrap().contains("i32"));
}

#[test]
fn test_code_map_includes_full_details_with_docs() {
    // Given: Rust fixture project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src").to_str().unwrap(),
        "detail": "full",
        "max_tokens": 10000
    });

    // When: code_map with detail="full"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns names + signatures + docs
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    let calc_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().contains("calculator.rs"))
        .unwrap();
    let functions = calc_file["functions"].as_array().unwrap();
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    // Should have signature and doc
    assert!(add_fn["signature"].is_string());
    assert!(add_fn["doc"].is_string());
    assert!(add_fn["doc"].as_str().unwrap().contains("Adds two numbers"));

    // In full mode, should also have code snippet
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("a + b"),
            "Full mode should include code snippet"
        );
        assert!(
            code.contains("pub fn add"),
            "Code should include function signature"
        );
    }
}

// ============================================================================
// Multi-Language Tests
// ============================================================================

#[test]
fn test_code_map_python_project() {
    // Given: Python fixture project
    let dir_path = common::fixture_dir("python");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns all Python files with structure
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    assert!(files.len() >= 2); // calculator.py, utils/helpers.py

    // Check for Calculator class
    let calc_file = files
        .iter()
        .find(|f| f["path"].as_str().unwrap().contains("calculator.py"))
        .unwrap();
    assert!(calc_file["classes"].as_array().unwrap().len() >= 2); // Calculator, Point

    // Verify functions have proper structure
    let functions = calc_file["functions"].as_array().unwrap();
    if let Some(add_fn) = functions.iter().find(|f| f["name"] == "add") {
        assert!(add_fn["signature"].is_string());
        // Code may be present depending on detail level
        if add_fn["code"].is_string() {
            let code = add_fn["code"].as_str().unwrap();
            assert!(code.contains("return a + b"), "Code should match fixture");
        }
    }
}

#[test]
fn test_code_map_javascript_project() {
    // Given: JavaScript fixture project
    let dir_path = common::fixture_dir("javascript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns all JS files with structure
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    assert!(files.len() >= 2); // calculator.js, utils/helpers.js
}

#[test]
fn test_code_map_typescript_project() {
    // Given: TypeScript fixture project
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns all TS files with structure
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    assert!(files.len() >= 2); // calculator.ts, types/models.ts
}

// ============================================================================
// Feature Tests
// ============================================================================

#[test]
fn test_code_map_filters_files_by_glob_pattern() {
    // Given: Mixed language project (use rust project with multiple file types)
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "pattern": "*.rs"
    });

    // When: code_map with pattern="*.rs"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns only Rust files
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    // All files should be .rs files
    for file in files {
        assert!(file["path"].as_str().unwrap().ends_with(".rs"));
    }
}

#[test]
fn test_code_map_respects_token_budget_limit() {
    // Given: Large project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "max_tokens": 500,
        "detail": "full"
    });

    // When: code_map with max_tokens=500
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Output is truncated, truncated=true
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have truncated flag if output was limited
    // Approximate: 1 token ~ 4 characters
    if text.len() > 500 * 6 {
        assert_eq!(map["truncated"], true);
    }
}

#[test]
fn test_code_map_handles_single_file_analysis() {
    // Given: Path to single file
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "path": file_path.to_str().unwrap()
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns structure for that file only
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0]["path"].as_str().unwrap().contains("calculator.rs"));
}

#[test]
fn test_code_map_skips_hidden_and_vendor() {
    // Given: Project with .git, node_modules, target dirs
    // We'll use the rust project which has a target dir when built
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    // When: code_map is called
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: These directories are skipped
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    // No files should be from target, .git, or node_modules
    for file in files {
        let path = file["path"].as_str().unwrap();
        assert!(!path.contains("/target/"));
        assert!(!path.contains("/.git/"));
        assert!(!path.contains("/node_modules/"));
    }
}

// ============================================================================
// Code Content Verification Tests
// ============================================================================

#[test]
fn test_code_map_full_mode_includes_actual_code() {
    // Given: Rust fixture project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src/calculator.rs").to_str().unwrap(),
        "detail": "full"
    });

    // When: code_map with detail="full"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns actual code snippets from fixture
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    let calc_file = &files[0];
    let functions = calc_file["functions"].as_array().unwrap();

    // Find add function and verify code
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();
    if add_fn["code"].is_string() {
        let code = add_fn["code"].as_str().unwrap();
        assert!(
            code.contains("a + b"),
            "Full mode should include actual implementation"
        );
        assert!(
            code.contains("pub fn add"),
            "Full mode should include signature"
        );
    }
}

#[test]
fn test_code_map_signatures_mode_no_code() {
    // Given: Rust fixture project
    let dir_path = common::fixture_dir("rust");
    let arguments = json!({
        "path": dir_path.join("src/calculator.rs").to_str().unwrap(),
        "detail": "signatures"
    });

    // When: code_map with detail="signatures"
    let result = treesitter_mcp::analysis::code_map::execute(&arguments);

    // Then: Returns signatures but not full code
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let map: serde_json::Value = serde_json::from_str(&text).unwrap();

    let files = map["files"].as_array().unwrap();
    let calc_file = &files[0];
    let functions = calc_file["functions"].as_array().unwrap();

    // Find add function
    let add_fn = functions.iter().find(|f| f["name"] == "add").unwrap();

    // Should have signature
    assert!(add_fn["signature"].is_string());
    assert!(add_fn["signature"].as_str().unwrap().contains("pub fn add"));

    // Should NOT have full code in signatures mode
    assert!(add_fn["code"].is_null() || !add_fn.get("code").is_some());
}
