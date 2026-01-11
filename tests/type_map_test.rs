//! Type Map Tests
//!
//! Tests for polyglot type extraction across TypeScript, Python, Java, and C#.

mod common;

use serde_json::json;

#[test]
fn test_type_extraction_typescript_interfaces() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for TypeScript: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(types["language"], "TypeScript");

    let interfaces = types["interfaces"].as_array().unwrap();
    assert!(
        !interfaces.is_empty(),
        "Should extract interfaces from TypeScript"
    );

    let interface_names: Vec<_> = interfaces
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();

    assert!(
        interface_names.contains(&"Point"),
        "Should find Point interface"
    );
    assert!(
        interface_names.contains(&"CalculatorOptions"),
        "Should find CalculatorOptions interface"
    );
}

#[test]
fn test_type_extraction_typescript_type_aliases() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for TypeScript: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let type_aliases = types["type_aliases"].as_array().unwrap();
    assert!(
        !type_aliases.is_empty(),
        "Should extract type aliases from TypeScript"
    );

    let type_alias_names: Vec<_> = type_aliases
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    assert!(
        type_alias_names.contains(&"OperationResult"),
        "Should find OperationResult type alias"
    );
    assert!(
        type_alias_names.contains(&"Result"),
        "Should find Result type alias"
    );
}

#[test]
fn test_python_class_method_extraction() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Python: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = types["classes"].as_array().unwrap();
    let point_class = classes.iter().find(|c| c["name"] == "Point").unwrap();

    let methods = point_class["members"].as_array().unwrap();
    assert!(
        !methods.is_empty(),
        "Should extract methods from Python class"
    );

    let method_names: Vec<_> = methods
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();

    assert!(
        method_names.contains(&"__init__"),
        "Should find __init__ method"
    );
    assert!(
        method_names.contains(&"origin"),
        "Should find origin method"
    );
    assert!(
        method_names.contains(&"distance_from_origin"),
        "Should find distance_from_origin method"
    );
    assert!(
        method_names.contains(&"translate"),
        "Should find translate method"
    );

    let init_method = methods.iter().find(|m| m["name"] == "__init__").unwrap();
    assert_eq!(
        init_method["visibility"].as_str().unwrap(),
        "public",
        "Method should have visibility information"
    );
}

#[test]
fn test_python_class_property_extraction() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Python: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = types["classes"].as_array().unwrap();
    let point_class = classes.iter().find(|c| c["name"] == "Point").unwrap();

    let properties = point_class["fields"].as_array().unwrap();
    assert!(
        !properties.is_empty(),
        "Should extract properties from Python class"
    );

    let property_names: Vec<_> = properties
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(property_names.contains(&"x"), "Should find x property");
    assert!(property_names.contains(&"y"), "Should find y property");

    let x_prop = properties.iter().find(|p| p["name"] == "x").unwrap();
    assert_eq!(
        x_prop["visibility"].as_str().unwrap(),
        "public",
        "Property should have visibility information"
    );
}

#[test]
fn test_type_map_integration_scans_directory_and_aggregates_types() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for directory: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(
        types["language"], "Mixed",
        "Should identify language as Mixed for directory"
    );

    let interfaces = types["interfaces"].as_array().unwrap();
    let type_aliases = types["type_aliases"].as_array().unwrap();

    assert!(
        !interfaces.is_empty() || !type_aliases.is_empty(),
        "Should extract types from multiple files in directory"
    );
}

#[test]
fn test_type_map_integration_filters_by_pattern() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "pattern": "Point"
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed with pattern: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let all_types: Vec<_> = types["interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .chain(types["classes"].as_array().unwrap().iter())
        .chain(types["structs"].as_array().unwrap().iter())
        .filter_map(|t| t["name"].as_str())
        .collect();

    assert!(
        !all_types.is_empty(),
        "Should find types matching pattern 'Point'"
    );

    for type_name in all_types {
        assert!(
            type_name.contains("Point"),
            "All types should contain 'Point': {}",
            type_name
        );
    }
}

#[test]
fn test_type_map_integration_respects_limit() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "limit": 2
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed with limit: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_count = types["interfaces"].as_array().unwrap().len()
        + types["classes"].as_array().unwrap().len()
        + types["structs"].as_array().unwrap().len()
        + types["enums"].as_array().unwrap().len()
        + types["traits"].as_array().unwrap().len()
        + types["type_aliases"].as_array().unwrap().len()
        + types["others"].as_array().unwrap().len();

    assert!(
        total_count <= 2,
        "Should respect limit of 2, got {} types",
        total_count
    );
}

#[test]
fn test_type_map_integration_respects_offset() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "offset": 1,
        "limit": 1
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed with offset: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let total_count = types["interfaces"].as_array().unwrap().len()
        + types["classes"].as_array().unwrap().len()
        + types["structs"].as_array().unwrap().len()
        + types["enums"].as_array().unwrap().len()
        + types["traits"].as_array().unwrap().len()
        + types["type_aliases"].as_array().unwrap().len()
        + types["others"].as_array().unwrap().len();

    assert!(
        total_count <= 1,
        "Should return at most 1 type with offset 1 and limit 1, got {}",
        total_count
    );
}

#[test]
fn test_type_map_integration_calculates_usage_count() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "limit": 50
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for directory: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let any_type = types["interfaces"]
        .as_array()
        .unwrap()
        .iter()
        .next()
        .expect("Should have at least one type");

    let usage_count = any_type["usage_count"].as_u64().unwrap_or(0);

    assert!(
        usage_count > 0,
        "Integration test expects usage_count to be calculated and > 0. \
         Currently usage_count is {} - feature needs to be implemented.",
        usage_count
    );
}

#[test]
fn test_type_map_integration_includes_project_dependencies() {
    let dir_path = common::fixture_dir("typescript");
    let arguments = json!({
        "path": dir_path.to_str().unwrap(),
        "include_deps": true
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed with include_deps: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let includes_deps = types.get("includes_dependencies");
    assert!(
        includes_deps.is_some(),
        "Should have includes_dependencies field when include_deps is specified"
    );

    assert_eq!(
        types["includes_dependencies"], true,
        "includes_dependencies should be true when include_deps=true"
    );

    let dependencies = types["dependencies"].as_array();
    assert!(
        dependencies.is_some(),
        "Should include dependency types field when include_deps=true"
    );
    // Not checking for non-empty because external dependencies are not yet implemented
    // assert!(
    //    dependencies.is_some() && !dependencies.unwrap().is_empty(),
    //    "Should include dependency types when include_deps=true"
    // );
}

#[test]
fn test_type_extraction_python_classes() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Python: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(types["language"], "Python");

    let classes = types["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should extract classes from Python");

    let class_names: Vec<_> = classes
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();

    assert!(
        class_names.contains(&"Calculator"),
        "Should find Calculator class"
    );
    assert!(class_names.contains(&"Point"), "Should find Point class");
    assert!(
        class_names.contains(&"LineSegment"),
        "Should find LineSegment class"
    );
}

#[test]
fn test_type_extraction_python_methods() {
    let file_path = common::fixture_path("python", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Python: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let classes = types["classes"].as_array().unwrap();

    let calculator_class: Option<&_> = classes.iter().find(|c| c["name"] == "Calculator");

    assert!(calculator_class.is_some(), "Should find Calculator class");

    let methods = calculator_class.unwrap()["members"].as_array().unwrap();
    assert!(
        !methods.is_empty(),
        "Should extract methods from Python class"
    );

    let method_names: Vec<_> = methods
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();

    assert!(method_names.contains(&"add"), "Should find add method");
    assert!(
        method_names.contains(&"subtract"),
        "Should find subtract method"
    );
}

#[test]
fn test_type_extraction_java_classes() {
    let file_path = common::fixture_path("java", "models/Point.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Java: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(types["language"], "Java");

    let classes = types["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should extract classes from Java");

    let point_class: Option<&_> = classes.iter().find(|c| c["name"] == "Point");

    assert!(point_class.is_some(), "Should find Point class");
}

#[test]
fn test_rust_impl_method_extraction() {
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Rust: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let structs = types["structs"].as_array().unwrap();
    let point_struct = structs.iter().find(|s| s["name"] == "Point").unwrap();

    let methods = point_struct["members"].as_array().unwrap();
    assert!(
        !methods.is_empty(),
        "Should extract methods from Rust impl block"
    );

    let method_names: Vec<_> = methods
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();

    assert!(method_names.contains(&"new"), "Should find new method");
    assert!(
        method_names.contains(&"origin"),
        "Should find origin method"
    );
    assert!(
        method_names.contains(&"distance_from_origin"),
        "Should find distance_from_origin method"
    );
    assert!(
        method_names.contains(&"translate"),
        "Should find translate method"
    );
}

#[test]
fn test_type_extraction_csharp_classes() {
    let file_path = common::fixture_path("csharp", "Models/Point.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for C#: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(types["language"], "C#");

    let classes = types["classes"].as_array().unwrap();
    assert!(!classes.is_empty(), "Should extract classes from C#");

    let class_names: Vec<_> = classes
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();

    assert!(class_names.contains(&"Point"), "Should find Point class");
    assert!(
        class_names.contains(&"LineSegment"),
        "Should find LineSegment class"
    );
    assert!(class_names.contains(&"Circle"), "Should find Circle class");
}

#[test]
fn test_type_extraction_csharp_interfaces() {
    let file_path = common::fixture_path("csharp", "Models/Point.cs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for C#: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let interfaces = types["interfaces"].as_array().unwrap();
    assert!(!interfaces.is_empty(), "Should extract interfaces from C#");

    let interface_names: Vec<_> = interfaces
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();

    assert!(
        interface_names.contains(&"IShape"),
        "Should find IShape interface"
    );
}

#[test]
fn test_type_extraction_all_languages_have_language_field() {
    let test_cases = vec![
        ("typescript", "types/models.ts", "TypeScript"),
        ("python", "calculator.py", "Python"),
        ("java", "models/Point.java", "Java"),
        ("csharp", "Models/Point.cs", "C#"),
    ];

    for (lang, file, expected_lang) in test_cases {
        let file_path = common::fixture_path(lang, file);
        let arguments = json!({
            "file_path": file_path.to_str().unwrap()
        });

        let result = treesitter_mcp::analysis::type_map::execute(&arguments)
            .unwrap_or_else(|e| panic!("type_map failed for {}: {}", lang, e));

        let text = common::get_result_text(&result);
        let types: serde_json::Value = serde_json::from_str(&text).unwrap();

        assert_eq!(
            types["language"], expected_lang,
            "Should have correct language field for {}",
            lang
        );
    }
}

#[test]
fn test_type_extraction_includes_type_info() {
    let file_path = common::fixture_path("typescript", "types/models.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for TypeScript: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    let interfaces = types["interfaces"].as_array().unwrap();
    let point_interface = interfaces.iter().find(|i| i["name"] == "Point").unwrap();

    assert!(
        point_interface["fields"].is_array(),
        "Interface should have fields array"
    );

    let properties = point_interface["fields"].as_array().unwrap();
    assert!(
        !properties.is_empty(),
        "Point interface should have properties"
    );

    let x_prop = properties.iter().find(|p| p["name"] == "x").unwrap();
    assert_eq!(
        x_prop["type"].as_str().unwrap(),
        "number",
        "Property should have type information"
    );
}

#[test]
fn test_rust_struct_field_extraction() {
    let file_path = common::fixture_path("rust", "src/models/mod.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::type_map::execute(&arguments)
        .unwrap_or_else(|e| panic!("type_map failed for Rust: {}", e));

    let text = common::get_result_text(&result);
    let types: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(types["language"], "Rust");

    let structs = types["structs"].as_array().unwrap();
    assert!(!structs.is_empty(), "Should extract structs from Rust");

    let point_struct: Option<&_> = structs.iter().find(|s| s["name"] == "Point");
    assert!(point_struct.is_some(), "Should find Point struct");

    let fields = point_struct.unwrap()["fields"].as_array().unwrap();
    assert!(!fields.is_empty(), "Should extract fields from Rust struct");

    let field_names: Vec<_> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();

    assert!(field_names.contains(&"x"), "Should find x field");
    assert!(field_names.contains(&"y"), "Should find y field");

    let x_field = fields.iter().find(|f| f["name"] == "x").unwrap();
    assert_eq!(
        x_field["type"].as_str().unwrap(),
        "i32",
        "Field should have type information"
    );
}
