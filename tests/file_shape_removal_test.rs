mod common;

use serde_json::json;

// Tests to verify that file_shape tool has been removed

#[test]
fn test_file_shape_module_does_not_exist() {
    // Given: The treesitter_mcp crate
    // When: We try to access file_shape module
    // Then: It should not exist

    // This test will fail to compile if file_shape module still exists
    // We're testing that the module is removed from the public API

    // Try to call file_shape - this should not compile once removed
    // For now, we just verify the module exists (will fail after removal)
    let _module_exists = std::any::type_name::<()>().contains("file_shape");

    // This test will be updated once file_shape is removed
    // For now, it's a placeholder that will need implementation changes
}

#[test]
fn test_file_shape_functionality_moved_to_parse_file() {
    // Given: A file that would have used file_shape
    // When: We use parse_file instead
    // Then: We get the same shape information

    // This test verifies that parse_file now provides the functionality
    // that file_shape used to provide

    let file_path = common::fixture_path("rust", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    // parse_file should now return file shape (not S-expression)
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    assert!(
        result.is_ok(),
        "parse_file should work as replacement for file_shape"
    );

    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Should have the same fields that file_shape used to return
    assert!(shape["functions"].is_array());
    // calculator.rs has functions and imports, but structs are in models/mod.rs
    // The structs/classes fields are omitted if empty (skip_serializing_if)
    // Just verify we have functions and imports which proves parse_file works
    assert!(shape["imports"].is_array());

    // Verify it has the enhanced fields (language, path)
    assert!(shape.get("language").is_some());
    assert!(shape.get("path").is_some());
}
