//! Helper functions for common test assertions

use serde_json::Value;

/// Assert that a shape has a function with the given name
pub fn assert_has_function(shape: &Value, name: &str) {
    let functions = shape["functions"]
        .as_array()
        .unwrap_or_else(|| panic!("Shape should have functions array"));

    let found = functions.iter().any(|f| f["name"] == name);
    assert!(
        found,
        "Should find function '{}' in {:?}",
        name,
        functions.iter().map(|f| &f["name"]).collect::<Vec<_>>()
    );
}

/// Assert that a function has code containing specific text
pub fn assert_function_code_contains(shape: &Value, func_name: &str, code_text: &str) {
    let functions = shape["functions"].as_array().unwrap();
    let func = functions
        .iter()
        .find(|f| f["name"] == func_name)
        .unwrap_or_else(|| panic!("Should find function '{}'", func_name));

    let code = func["code"]
        .as_str()
        .unwrap_or_else(|| panic!("Function '{}' should have code", func_name));

    assert!(
        code.contains(code_text),
        "Function '{}' code should contain '{}', got:\n{}",
        func_name,
        code_text,
        code
    );
}

/// Assert that all paths in a result are relative (no absolute markers)
pub fn assert_all_paths_relative(value: &Value, path_field: &str) {
    let items = value
        .as_array()
        .or_else(|| value[path_field].as_array())
        .unwrap();

    for item in items {
        let path = item["path"]
            .as_str()
            .or_else(|| item["file"].as_str())
            .unwrap();

        assert!(
            !path.contains("/Users/") && !path.contains("/home/") && !path.starts_with("C:\\"),
            "Path should be relative, got: {}",
            path
        );
    }
}

/// Assert minimum number of items in an array field
pub fn assert_min_count(value: &Value, field: &str, min: usize) {
    let items = value[field]
        .as_array()
        .unwrap_or_else(|| panic!("Should have '{}' array", field));

    assert!(
        items.len() >= min,
        "Should have at least {} items in '{}', got {}",
        min,
        field,
        items.len()
    );
}
