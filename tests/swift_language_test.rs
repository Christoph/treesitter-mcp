use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{detect_language, parse_code, Language};

/// Test suite for Swift language support
///
/// These tests verify that the tree-sitter parser can detect and parse
/// Swift source files (.swift extension).

#[test]
fn test_detect_language_from_swift_file() {
    let lang = detect_language("ViewController.swift").unwrap();
    assert_eq!(lang, Language::Swift);
}

#[test]
fn test_detect_language_from_uppercase_swift_file() {
    let lang = detect_language("Main.SWIFT").unwrap();
    assert_eq!(lang, Language::Swift);
}

#[test]
fn test_swift_language_has_correct_name() {
    assert_eq!(Language::Swift.name(), "Swift");
}

/// Test extraction of Swift functions
#[test]
fn test_extract_swift_function() {
    let source = r#"
/// Adds two numbers together
func add(a: Int, b: Int) -> Int {
    return a + b
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.functions.len(), 1, "Expected 1 function");
    let func = &shape.functions[0];
    assert_eq!(func.name, "add");
    assert!(
        func.signature.contains("func add"),
        "Expected signature to contain 'func add', got: {}",
        func.signature
    );
    assert_eq!(func.line, 3, "Expected function to start at line 3");
}

/// Test extraction of Swift class with methods
#[test]
fn test_extract_swift_class_with_methods() {
    let source = r#"
/// A simple calculator class
class Calculator {
    /// Adds two numbers
    func add(a: Int, b: Int) -> Int {
        return a + b
    }
    
    /// Subtracts b from a
    func subtract(a: Int, b: Int) -> Int {
        return a - b
    }
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.classes.len(), 1, "Expected 1 class");
    let class = &shape.classes[0];
    assert_eq!(class.name, "Calculator");
    assert_eq!(class.line, 3, "Expected class to start at line 3");
    assert_eq!(
        class.methods.len(),
        2,
        "Expected 2 methods in Calculator class"
    );

    let method1 = &class.methods[0];
    assert_eq!(method1.name, "add");
    assert!(method1.signature.contains("func add"));

    let method2 = &class.methods[1];
    assert_eq!(method2.name, "subtract");
    assert!(method2.signature.contains("func subtract"));
}

/// Test extraction of Swift struct
#[test]
fn test_extract_swift_struct() {
    let source = r#"
/// Represents a point in 2D space
struct Point {
    let x: Double
    let y: Double
    
    /// Calculate distance from origin
    func distanceFromOrigin() -> Double {
        return sqrt(x * x + y * y)
    }
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.structs.len(), 1, "Expected 1 struct");
    let struct_info = &shape.structs[0];
    assert_eq!(struct_info.name, "Point");
    assert_eq!(struct_info.line, 3, "Expected struct to start at line 3");
    assert!(
        struct_info.code.as_ref().unwrap().contains("struct Point"),
        "Expected code to contain 'struct Point'"
    );
}

/// Test extraction of Swift protocol (similar to interface/trait)
#[test]
fn test_extract_swift_protocol() {
    let source = r#"
/// Protocol for objects that can be described
protocol Describable {
    func describe() -> String
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    // Protocols should be extracted as traits/interfaces
    assert_eq!(
        shape.traits.len(),
        1,
        "Expected 1 protocol (extracted as trait)"
    );
    let protocol = &shape.traits[0];
    assert_eq!(protocol.name, "Describable");
    assert!(
        protocol.doc.is_some()
            || shape
                .functions
                .iter()
                .any(|f| f.name.contains("Describable")),
        "Expected protocol information to be captured"
    );
}

/// Test extraction of Swift imports
#[test]
fn test_extract_swift_imports() {
    let source = r#"
import Foundation
import UIKit
import SwiftUI

class MyViewController {
    func hello() {
        print("Hello")
    }
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.imports.len(), 3, "Expected 3 imports");
    assert!(
        shape.imports.iter().any(|i| i.text.contains("Foundation")),
        "Expected Foundation import"
    );
    assert!(
        shape.imports.iter().any(|i| i.text.contains("UIKit")),
        "Expected UIKit import"
    );
    assert!(
        shape.imports.iter().any(|i| i.text.contains("SwiftUI")),
        "Expected SwiftUI import"
    );
}

/// Test extraction of Swift class with doc comments
#[test]
fn test_extract_swift_with_doc_comments() {
    let source = r#"
/// A user model representing application users
class User {
    /// Returns the full name of the user
    func fullName() -> String {
        return "John Doe"
    }
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.classes.len(), 1);
    let class = &shape.classes[0];
    assert!(class.doc.is_some(), "Expected class to have doc comment");
    assert!(class.doc.as_ref().unwrap().contains("user model"));

    assert_eq!(class.methods.len(), 1);
    let method = &class.methods[0];
    assert!(method.doc.is_some(), "Expected method to have doc comment");
    assert!(method.doc.as_ref().unwrap().contains("full name"));
}

/// Test that include_code parameter controls code inclusion
#[test]
fn test_extract_swift_without_code() {
    let source = r#"
func calculate() -> Int {
    let result = 42
    return result
}
"#;
    let tree = parse_code(source, Language::Swift).expect("Failed to parse Swift");
    let shape = extract_enhanced_shape(&tree, source, Language::Swift, None, false)
        .expect("Failed to extract shape");

    assert_eq!(shape.functions.len(), 1);
    let func = &shape.functions[0];
    assert!(
        func.code.is_none(),
        "Expected code to be None when include_code is false"
    );
    assert!(
        !func.signature.is_empty(),
        "Expected signature to be present even without code"
    );
}
