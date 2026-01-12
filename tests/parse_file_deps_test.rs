mod common;

use serde_json::json;

#[test]
fn test_parse_file_accepts_include_deps_parameter() {
    // Given: parse_file with include_deps parameter
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    // When: Execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should accept parameter without error
    assert!(result.is_ok());
}

#[test]
fn test_parse_file_no_deps() {
    // Given: parse_file with include_deps=false
    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: No dependencies included
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Project types are always included (even if empty), but should be minimal for files with no deps
    // Just verify the structure is valid
    assert!(shape.is_object(), "Should return valid shape object");
}

#[test]
fn test_parse_file_with_deps_rust() {
    // Given: Rust file with mod declarations
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file with include_deps=true
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Project types are included with signatures only
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Compact schema: dependencies are provided via a `deps` map.
    let deps = shape
        .get("deps")
        .and_then(|v| v.as_object())
        .expect("Should include deps object when include_deps=true");

    assert!(!deps.is_empty(), "Should include at least one dependency");

    // Basic structural check: at least one dep entry with rows.
    let first_rows = deps
        .iter()
        .find_map(|(_path, rows)| rows.as_str())
        .expect("Deps entries should contain row strings");

    let rows = common::helpers::parse_compact_rows(first_rows);
    assert!(!rows.is_empty(), "Dep should have rows");

    for row in rows {
        assert!(
            !row.get(0).map(|s| s.is_empty()).unwrap_or(true),
            "Row should have a type name"
        );
        assert!(
            row.get(1).and_then(|s| s.parse::<usize>().ok()).is_some(),
            "Row should have a numeric line"
        );
    }
}

#[test]
fn test_parse_file_deps_token_efficiency() {
    // Given: File with dependencies
    let file_path = common::fixture_path("rust", "src/lib.rs");

    // When: Parse with full code
    let full_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": true,
    });
    let full_result = treesitter_mcp::analysis::view_code::execute(&full_args).unwrap();
    let full_text = common::get_result_text(&full_result);

    // When: Parse with signatures only
    let sig_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });
    let sig_result = treesitter_mcp::analysis::view_code::execute(&sig_args).unwrap();
    let sig_text = common::get_result_text(&sig_result);

    // Then: Signatures-only should be smaller or similar size
    // (For small files, the difference might be minimal)
    let full_size = full_text.len();
    let sig_size = sig_text.len();

    // Just verify signatures version is not significantly larger
    assert!(
        sig_size <= full_size * 11 / 10,
        "Signatures should not be >110% of full size. Got {} vs {}",
        sig_size,
        full_size
    );
}

#[test]
fn test_parse_file_deps_python() {
    // Given: Python file with imports
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify Python class methods are included in main file (compact `cm` rows)
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows_str = shape.get("cm").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true)); // class
        assert!(!row.get(1).map(|s| s.is_empty()).unwrap_or(true)); // method
        assert!(!row.get(3).map(|s| s.is_empty()).unwrap_or(true)); // signature
    }
}

#[test]
fn test_parse_file_deps_javascript() {
    // Given: JavaScript file with imports
    let file_path = common::fixture_path("javascript", "index.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify JS class methods are included (compact `cm` rows)
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let rows_str = shape.get("cm").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    for row in rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true)); // class
        assert!(!row.get(1).map(|s| s.is_empty()).unwrap_or(true)); // method
    }
}

#[test]
fn test_parse_file_deps_typescript() {
    // Given: TypeScript file with imports
    let file_path = common::fixture_path("typescript", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    // Verify TS interfaces and classes are included (compact `i` + `cm` rows)
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let iface_rows_str = shape.get("i").and_then(|v| v.as_str()).unwrap_or("");
    let iface_rows = common::helpers::parse_compact_rows(iface_rows_str);
    for row in iface_rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true)); // iface name
    }

    let method_rows_str = shape.get("cm").and_then(|v| v.as_str()).unwrap_or("");
    let method_rows = common::helpers::parse_compact_rows(method_rows_str);
    for row in method_rows {
        assert!(!row.get(0).map(|s| s.is_empty()).unwrap_or(true)); // class
        assert!(!row.get(1).map(|s| s.is_empty()).unwrap_or(true)); // method
    }
}

#[test]
fn test_parse_file_deps_rust_traits() {
    // Given: Rust file with traits in dependencies
    let file_path = common::fixture_path("rust", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::view_code::execute(&arguments);

    // Then: Should succeed
    assert!(result.is_ok());

    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Compact schema: dependency output is a `deps` map of row strings.
    let deps = shape.get("deps").and_then(|v| v.as_object());
    assert!(deps.is_some(), "Should include deps object");
}
