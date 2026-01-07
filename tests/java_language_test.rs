mod common;

use serde_json::json;

/// Test that parse_file extracts static methods from Java files
///
/// Verifies that the analyzer can identify and extract static method declarations
/// from Java classes, including method names, signatures, and documentation.
#[test]
fn test_parse_java_extracts_static_methods() {
    let file_path = common::fixture_path("java", "Calculator.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify language detection
    assert_eq!(shape["language"], "Java", "Should detect Java language");

    // Verify static methods from Calculator class
    let expected_methods = vec!["add", "subtract", "multiply", "divide", "applyOperation"];
    for method_name in expected_methods {
        common::helpers::assert_has_function(&shape, method_name);
    }
}

/// Test that parse_file extracts classes from Java files
///
/// Verifies that the analyzer can identify both public and package-private
/// classes in Java files, extracting their names and structure.
#[test]
fn test_parse_java_extracts_classes() {
    let file_path = common::fixture_path("java", "Calculator.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify classes are extracted
    let classes = shape["classes"]
        .as_array()
        .expect("Should have classes array");

    let class_names: Vec<&str> = classes.iter().filter_map(|c| c["name"].as_str()).collect();

    assert!(
        class_names.contains(&"Calculator"),
        "Should find Calculator class, found: {:?}",
        class_names
    );

    assert!(
        class_names.contains(&"CalculatorState"),
        "Should find CalculatorState class, found: {:?}",
        class_names
    );
}

/// Test that parse_file extracts instance methods from Java classes
///
/// Verifies that the analyzer can identify instance (non-static) methods
/// from Java classes, including getters, setters, and business logic methods.
#[test]
fn test_parse_java_extracts_instance_methods() {
    let file_path = common::fixture_path("java", "Calculator.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Check that CalculatorState class has expected instance methods
    let expected_methods = vec![
        "getValue",
        "setValue",
        "add",
        "subtract",
        "reset",
        "getHistory",
        "hasHistory",
    ];
    for method_name in expected_methods {
        common::helpers::assert_has_function(&shape, method_name);
    }
}

/// Test that parse_file extracts interfaces from Java files
///
/// Verifies that the analyzer can identify Java interface declarations
/// and their method signatures.
#[test]
fn test_parse_java_extracts_interfaces() {
    let file_path = common::fixture_path("java", "models/Shape.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify interfaces are extracted
    let interfaces = shape["interfaces"]
        .as_array()
        .expect("Should have interfaces array");

    let interface_names: Vec<&str> = interfaces
        .iter()
        .filter_map(|i| i["name"].as_str())
        .collect();

    assert!(
        interface_names.contains(&"Shape"),
        "Should find Shape interface, found: {:?}",
        interface_names
    );
}

/// Test that parse_file handles interface implementation
///
/// Verifies that the analyzer can identify classes that implement interfaces
/// and properly extract the relationship between them.
#[test]
fn test_parse_java_handles_interface_implementation() {
    let file_path = common::fixture_path("java", "models/Shape.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify Circle class implements Shape interface
    let classes = shape["classes"]
        .as_array()
        .expect("Should have classes array");

    let circle = classes
        .iter()
        .find(|c| c["name"] == "Circle")
        .expect("Should find Circle class");

    // Check that Circle implements the interface methods
    assert!(
        circle["implements"]
            .as_array()
            .map(|arr| arr.iter().any(|i| i == "Shape"))
            .unwrap_or(false),
        "Circle should implement Shape interface"
    );
}

/// Test that parse_file extracts annotations from Java methods
///
/// Verifies that the analyzer can identify Java annotations like @Override
/// and include them in the extracted metadata.
#[test]
fn test_parse_java_extracts_annotations() {
    let file_path = common::fixture_path("java", "models/Shape.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Debug output
    println!("\n=== Parsed Shape ===");
    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            println!("\nClass: {}", class["name"].as_str().unwrap_or("?"));
            if let Some(methods) = class["methods"].as_array() {
                for method in methods {
                    println!(
                        "  Method: {} - annotations: {:?}",
                        method["name"].as_str().unwrap_or("?"),
                        method.get("annotations")
                    );
                }
            }
        }
    }

    // Verify that methods with @Override annotation are detected
    // In Java, @Override annotations are on class methods, not top-level functions
    let classes = shape["classes"]
        .as_array()
        .expect("Should have classes array");

    let has_override_annotation = classes.iter().any(|c| {
        c["methods"]
            .as_array()
            .map(|methods| {
                methods.iter().any(|m| {
                    m["annotations"]
                        .as_array()
                        .map(|arr| arr.iter().any(|a| a.as_str() == Some("Override")))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    });

    assert!(
        has_override_annotation,
        "Should find class methods with @Override annotation"
    );
}

/// Test that find_usages locates Java method calls
///
/// Verifies that the analyzer can find all usages of a method,
/// including its definition and call sites across a Java file.
#[test]
fn test_find_usages_locates_java_method_calls() {
    let file_path = common::fixture_path("java", "Calculator.java");
    let arguments = json!({
        "symbol": "add",
        "path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::find_usages::execute(&arguments)
        .expect("find_usages should succeed for Java file");

    let text = common::get_result_text(&result);
    let usages: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Should find at least the definition
    let usage_list = usages["usages"]
        .as_array()
        .expect("Should have usages array");

    assert!(
        usage_list.len() > 0,
        "Should find at least one usage of add method"
    );

    // Verify at least one usage is the definition
    let has_definition = usage_list.iter().any(|u| u["usage_type"] == "definition");

    assert!(has_definition, "Should find the definition of add method");
}

/// Test that parse_file extracts imports from Java files
///
/// Verifies that the analyzer can identify and extract import statements
/// from Java files, including both single-class and wildcard imports.
#[test]
fn test_parse_java_extracts_imports() {
    let file_path = common::fixture_path("java", "services/MathService.java");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::view_code::execute(&arguments)
        .expect("parse_file should succeed for Java file");

    let text = common::get_result_text(&result);
    let shape: serde_json::Value =
        serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify imports are extracted
    let imports = shape["imports"]
        .as_array()
        .expect("Should have imports array");

    let import_paths: Vec<&str> = imports.iter().filter_map(|i| i["text"].as_str()).collect();

    assert!(
        import_paths.iter().any(|p| p.contains("Point")),
        "Should find Point import, found: {:?}",
        import_paths
    );

    assert!(
        import_paths.iter().any(|p| p.contains("List")),
        "Should find List import, found: {:?}",
        import_paths
    );
}

/// Test that code_map provides overview of Java directory
///
/// Verifies that the analyzer can generate a comprehensive overview
/// of a Java project directory, including all classes, interfaces, and methods.
#[test]
fn test_code_map_provides_java_overview() {
    let dir_path = common::fixture_dir("java");
    let arguments = json!({
        "path": dir_path.to_str().unwrap()
    });

    let result = treesitter_mcp::analysis::code_map::execute(&arguments)
        .expect("code_map should succeed for Java directory");

    let text = common::get_result_text(&result);
    let map: serde_json::Value = serde_json::from_str(&text).expect("Result should be valid JSON");

    // Verify Java files are included
    let files = map["files"].as_array().expect("Should have files array");

    let has_calculator = files
        .iter()
        .any(|f| f["path"].as_str().unwrap().contains("Calculator.java"));

    let has_point = files
        .iter()
        .any(|f| f["path"].as_str().unwrap().contains("Point.java"));

    let has_shape = files
        .iter()
        .any(|f| f["path"].as_str().unwrap().contains("Shape.java"));

    let has_math_service = files
        .iter()
        .any(|f| f["path"].as_str().unwrap().contains("MathService.java"));

    assert!(has_calculator, "Should include Calculator.java in code map");
    assert!(has_point, "Should include Point.java in code map");
    assert!(has_shape, "Should include Shape.java in code map");
    assert!(
        has_math_service,
        "Should include MathService.java in code map"
    );
}
