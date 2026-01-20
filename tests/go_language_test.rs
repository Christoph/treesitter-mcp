use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{detect_language, parse_code, Language};

/// Test suite for Go language support
///
/// These tests verify that the tree-sitter parser can detect and parse
/// Go source files (.go extension).

#[test]
fn test_detect_language_from_go_file() {
    let lang = detect_language("main.go").unwrap();
    assert_eq!(lang, Language::Go);
}

#[test]
fn test_detect_language_from_uppercase_go_file() {
    let lang = detect_language("Main.GO").unwrap();
    assert_eq!(lang, Language::Go);
}

#[test]
fn test_go_language_has_correct_name() {
    assert_eq!(Language::Go.name(), "Go");
}

/// Test extraction of Go functions
#[test]
fn test_extract_go_function() {
    let source = r#"
package main

// Add adds two numbers together
func Add(a, b int) int {
    return a + b
}
"#;
    let tree = parse_code(source, Language::Go).expect("Failed to parse Go");
    let shape = extract_enhanced_shape(&tree, source, Language::Go, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.functions.len(), 1, "Expected 1 function");
    let func = &shape.functions[0];
    assert_eq!(func.name, "Add");
    assert!(
        func.signature.contains("func Add"),
        "Expected signature to contain 'func Add', got: {}",
        func.signature
    );
    assert_eq!(func.line, 5, "Expected function to start at line 5");
}

/// Test extraction of Go struct with methods
#[test]
fn test_extract_go_struct_with_methods() {
    let source = r#"
package main

// Calculator represents a simple calculator
type Calculator struct {
    result float64
}

// Add adds a value to the calculator's result
func (c *Calculator) Add(v float64) {
    c.result += v
}

// GetResult returns the current result
func (c *Calculator) GetResult() float64 {
    return c.result
}
"#;
    let tree = parse_code(source, Language::Go).expect("Failed to parse Go");
    let shape = extract_enhanced_shape(&tree, source, Language::Go, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.structs.len(), 1, "Expected 1 struct");
    let struct_info = &shape.structs[0];
    assert_eq!(struct_info.name, "Calculator");
    assert_eq!(struct_info.line, 5, "Expected struct to start at line 5");
    assert!(
        struct_info
            .code
            .as_ref()
            .unwrap()
            .contains("type Calculator"),
        "Expected code to contain 'type Calculator'"
    );
}

/// Test extraction of Go package imports
#[test]
fn test_extract_go_imports() {
    let source = r#"
package main

import (
    "fmt"
    "net/http"
    "os"
)

func main() {
    fmt.Println("Hello")
}
"#;
    let tree = parse_code(source, Language::Go).expect("Failed to parse Go");
    let shape = extract_enhanced_shape(&tree, source, Language::Go, None, true)
        .expect("Failed to extract shape");

    assert_eq!(shape.imports.len(), 3, "Expected 3 imports");
    assert!(
        shape.imports.iter().any(|i| i.text.contains("fmt")),
        "Expected fmt import"
    );
    assert!(
        shape.imports.iter().any(|i| i.text.contains("net/http")),
        "Expected net/http import"
    );
    assert!(
        shape.imports.iter().any(|i| i.text.contains("os")),
        "Expected os import"
    );
}

/// Test extraction of Go interface
#[test]
fn test_extract_go_interface() {
    let source = r#"
package main

// Shape represents a geometric shape
type Shape interface {
    Area() float64
    Perimeter() float64
}
"#;
    let tree = parse_code(source, Language::Go).expect("Failed to parse Go");
    let shape = extract_enhanced_shape(&tree, source, Language::Go, None, true)
        .expect("Failed to extract shape");

    // Interfaces should be extracted as traits/interfaces
    assert_eq!(
        shape.traits.len(),
        1,
        "Expected 1 interface (extracted as trait)"
    );
    let interface = &shape.traits[0];
    assert_eq!(interface.name, "Shape");
}

/// Test that tree_sitter_language() returns a valid grammar for Go
#[test]
fn test_tree_sitter_language_go_returns_valid_grammar() {
    let lang = Language::Go;
    let _ts_lang = lang.tree_sitter_language();

    // Verify we can parse a simple Go program with the grammar
    let source = r#"
package main

func main() {
    println("Hello, World!")
}
"#;

    let tree = parse_code(source, lang).unwrap();
    assert!(!tree.root_node().has_error());
}

/// Test that Go grammar can parse struct declarations
#[test]
fn test_tree_sitter_language_go_parses_structs() {
    let lang = Language::Go;
    let source = r#"
package main

type Person struct {
    Name string
    Age  int
}
"#;

    let tree = parse_code(source, lang).unwrap();
    let root = tree.root_node();

    // Verify the tree has a type declaration
    assert!(!root.has_error());
    assert!(root.to_sexp().contains("type_declaration"));
}

/// Test that Go grammar can parse methods
#[test]
fn test_tree_sitter_language_go_parses_methods() {
    let lang = Language::Go;
    let source = r#"
package main

type MyStruct struct {}

func (m *MyStruct) MyMethod() {
    // method body
}
"#;

    let tree = parse_code(source, lang).unwrap();
    let root = tree.root_node();

    // Verify the tree has a method declaration
    assert!(!root.has_error());
    assert!(root.to_sexp().contains("method_declaration"));
}
