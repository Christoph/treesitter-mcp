/// Tests for C# language support
///
/// This test suite verifies that treesitter-mcp correctly parses C# code,
/// extracting classes, methods, properties, interfaces, and other structural elements.
mod common;

use serde_json::json;

#[test]
fn test_parse_csharp_extracts_static_methods() {
    let file_path = common::fixture_path("csharp", "Calculator.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify static methods from Calculator class
    let expected_methods = vec!["Add", "Subtract", "Multiply", "Divide", "ApplyOperation"];
    for method_name in expected_methods {
        common::helpers::assert_has_function(&shape, method_name);
    }
}

#[test]
fn test_parse_csharp_extracts_classes() {
    let file_path = common::fixture_path("csharp", "Calculator.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify classes are extracted (compact `c` rows)
    let rows_str = shape.get("c").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let class_names: Vec<&str> = rows
        .iter()
        .filter_map(|r| r.first().map(|s| s.as_str()))
        .collect();

    assert!(
        class_names.contains(&"Calculator"),
        "Should find Calculator static class, found: {:?}",
        class_names
    );

    assert!(
        class_names.contains(&"CalculatorState"),
        "Should find CalculatorState class, found: {:?}",
        class_names
    );
}

#[test]
fn test_parse_csharp_extracts_class_methods() {
    let file_path = common::fixture_path("csharp", "Calculator.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Check that CalculatorState class has expected methods
    let expected_methods = vec!["Add", "Subtract", "Reset", "GetHistory"];
    for method_name in expected_methods {
        common::helpers::assert_has_function(&shape, method_name);
    }
}

#[test]
fn test_parse_csharp_extracts_properties() {
    let file_path = common::fixture_path("csharp", "Models/Point.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify properties are extracted (compact `pr` rows)
    let rows_str = shape.get("pr").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let property_names: Vec<&str> = rows
        .iter()
        .filter_map(|r| r.first().map(|s| s.as_str()))
        .collect();

    assert!(
        property_names.contains(&"X"),
        "Should find X property, found: {:?}",
        property_names
    );

    assert!(
        property_names.contains(&"Y"),
        "Should find Y property, found: {:?}",
        property_names
    );
}

#[test]
fn test_parse_csharp_extracts_interfaces() {
    let file_path = common::fixture_path("csharp", "Models/Point.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify interfaces are extracted (compact `i` rows)
    let rows_str = shape.get("i").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let interface_names: Vec<&str> = rows
        .iter()
        .filter_map(|r| r.first().map(|s| s.as_str()))
        .collect();

    assert!(
        interface_names.contains(&"IShape"),
        "Should find IShape interface, found: {:?}",
        interface_names
    );
}

#[test]
fn test_parse_csharp_handles_interface_implementation() {
    let file_path = common::fixture_path("csharp", "Models/Point.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "detail": "signatures",
        "include_deps": false,
        "max_tokens": 10_000
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for C# file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify Circle class implements IShape interface (compact `ci` rows)
    let rows_str = shape.get("ci").and_then(|v| v.as_str()).unwrap_or("");
    let rows = common::helpers::parse_compact_rows(rows_str);

    let circle_implements = rows.iter().any(|r| {
        r.get(0).map(|s| s.as_str()) == Some("Circle")
            && r.get(1).map(|s| s.as_str()) == Some("IShape")
    });

    assert!(
        circle_implements,
        "Circle should implement IShape interface"
    );
}

#[test]
fn test_find_usages_locates_csharp_method_calls() {
    let file_path = common::fixture_path("csharp", "Calculator.cs");
    let arguments = json!({
        "symbol": "Add",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments)
        .expect("find_usages should succeed for C# file");

    let text = common::get_result_text(&result);
    let usages: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    let rows = common::helpers::find_usages_rows(&usages);

    assert!(
        !rows.is_empty(),
        "Should find at least one usage of Add method"
    );

    let has_definition = rows
        .iter()
        .any(|row| row.get(3).map(|s| s.as_str()) == Some("definition"));

    assert!(has_definition, "Should find the definition of Add method");
}

#[test]
fn test_code_map_provides_csharp_overview() {
    let dir_path = common::fixture_dir("csharp");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments)
        .expect("code_map should succeed for C# directory");

    let text = common::get_result_text(&result);
    let map: serde_json::Value = serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify C# files are included
    let files = common::helpers::code_map_files(&map);

    let has_calculator = files.iter().any(|(p, _)| p.contains("Calculator.cs"));
    let has_point = files.iter().any(|(p, _)| p.contains("Point.cs"));

    assert!(has_calculator, "Should include Calculator.cs in code map");
    assert!(has_point, "Should include Point.cs in code map");
}
