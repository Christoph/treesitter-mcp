use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{parse_code, Language};

#[test]
fn test_enhanced_shape_json_format() {
    let source = r#"
/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// A simple calculator struct
pub struct Calculator {
    value: i32,
}

use std::fmt;
"#;

    let tree = parse_code(source, Language::Rust).expect("Failed to parse");
    let shape = extract_enhanced_shape(
        &tree,
        source,
        Language::Rust,
        Some("src/calculator.rs"),
        true,
    )
    .expect("Failed to extract shape");

    // Verify path and language
    assert_eq!(shape.path, Some("src/calculator.rs".to_string()));
    assert_eq!(shape.language, Some("Rust".to_string()));

    // Verify functions
    assert_eq!(shape.functions.len(), 1);
    let func = &shape.functions[0];
    assert_eq!(func.name, "add");
    assert!(func.signature.contains("pub fn add"));
    assert_eq!(func.line, 3); // Line 3 in the raw string (after the opening """)
    assert_eq!(func.end_line, 5);
    assert!(func.doc.is_some());
    assert!(func.code.is_some());

    // Verify structs (at least one should be Calculator)
    assert!(shape.structs.len() >= 1);
    let calculator = shape
        .structs
        .iter()
        .find(|s| s.name == "Calculator")
        .expect("Calculator struct not found");
    assert_eq!(calculator.line, 8); // Line 8 in the raw string
    assert!(calculator.doc.is_some());
    assert!(calculator.code.is_some());

    // Verify imports
    assert_eq!(shape.imports.len(), 1);
    assert_eq!(shape.imports[0].text, "use std::fmt;");
    assert_eq!(shape.imports[0].line, 12); // Line 12 in the raw string

    // Verify JSON serialization
    let json = serde_json::to_string(&shape).expect("Failed to serialize");
    assert!(json.contains("\"path\""));
    assert!(json.contains("\"language\""));
    assert!(json.contains("\"functions\""));
    assert!(json.contains("\"structs\""));
    assert!(json.contains("\"imports\""));
}

#[test]
fn test_python_enhanced_shape() {
    let source = r#"
def greet(name):
    """Greets a person"""
    return f"Hello, {name}!"

class Person:
    """Represents a person"""
    def __init__(self, name):
        self.name = name

from typing import List
"#;

    let tree = parse_code(source, Language::Python).expect("Failed to parse");
    let shape = extract_enhanced_shape(&tree, source, Language::Python, Some("greet.py"), true)
        .expect("Failed to extract shape");

    // Functions: greet (top-level) and __init__ (nested in class)
    assert!(shape.functions.len() >= 1);
    let greet = shape
        .functions
        .iter()
        .find(|f| f.name == "greet")
        .expect("greet function not found");
    assert_eq!(greet.line, 2);

    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.imports.len(), 1);

    let cls = &shape.classes[0];
    assert_eq!(cls.name, "Person");
    assert_eq!(cls.line, 6);
}

#[test]
fn test_javascript_enhanced_shape() {
    let source = r#"
function add(a, b) {
    return a + b;
}

class Calculator {
    multiply(a, b) {
        return a * b;
    }
}

import { utils } from './utils.js';
"#;

    let tree = parse_code(source, Language::JavaScript).expect("Failed to parse");
    let shape = extract_enhanced_shape(&tree, source, Language::JavaScript, Some("calc.js"), true)
        .expect("Failed to extract shape");

    assert_eq!(shape.functions.len(), 1);
    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.imports.len(), 1);

    let func = &shape.functions[0];
    assert_eq!(func.name, "add");
    assert_eq!(func.line, 2);

    let cls = &shape.classes[0];
    assert_eq!(cls.name, "Calculator");
    assert_eq!(cls.line, 6);
}
