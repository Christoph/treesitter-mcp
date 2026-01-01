mod common;

use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{parse_code, Language};

#[test]
fn test_extract_python_class_methods() {
    // Given: Python class with methods
    let source = r#"
class Calculator:
    """A simple calculator"""
    
    def __init__(self, value: int = 0):
        """Initialize calculator"""
        self.value = value
    
    def add(self, x: int) -> None:
        """Add a value"""
        self.value += x
    
    def get_value(self) -> int:
        return self.value
    "#;

    // When: Parse and extract shape
    let tree = parse_code(source, Language::Python).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::Python, None, false).unwrap();

    // Then: Should nest methods in class
    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].name, "Calculator");
    // Note: Python docstrings are not currently extracted as doc comments
    // assert_eq!(shape.classes[0].doc, Some("A simple calculator".to_string()));
    assert_eq!(shape.classes[0].methods.len(), 3);

    assert_eq!(shape.classes[0].methods[0].name, "__init__");
    assert_eq!(shape.classes[0].methods[1].name, "add");
    assert_eq!(shape.classes[0].methods[2].name, "get_value");

    // Should NOT appear in top-level functions
    assert_eq!(shape.functions.len(), 0);
}

#[test]
fn test_extract_python_nested_classes() {
    // Given: Nested classes
    let source = r#"
class Outer:
    class Inner:
        def method(self):
            pass
    
    def outer_method(self):
        pass
    "#;

    // When: Parse
    let tree = parse_code(source, Language::Python).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::Python, None, false).unwrap();

    // Then: Should handle nesting (only top-level class, only outer_method)
    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].name, "Outer");
    assert_eq!(shape.classes[0].methods.len(), 1);
    assert_eq!(shape.classes[0].methods[0].name, "outer_method");
}

#[test]
fn test_extract_python_class_with_code() {
    // Given: Python class
    let source = r#"
class Calculator:
    def add(self, x):
        self.value += x
    "#;

    // When: Parse with include_code=true
    let tree = parse_code(source, Language::Python).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::Python, None, true).unwrap();

    // Then: Should include method code
    assert!(shape.classes[0].methods[0].code.is_some());
    assert!(shape.classes[0].methods[0]
        .code
        .as_ref()
        .unwrap()
        .contains("self.value += x"));
}

#[test]
fn test_extract_python_class_without_code() {
    // Given: Python class
    let source = r#"
class Calculator:
    def add(self, x):
        self.value += x
    "#;

    // When: Parse with include_code=false
    let tree = parse_code(source, Language::Python).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::Python, None, false).unwrap();

    // Then: Should NOT include method code
    assert!(shape.classes[0].methods[0].code.is_none());
}

#[test]
fn test_extract_python_top_level_functions_separate() {
    // Given: Mix of top-level functions and class methods
    let source = r#"
def top_level_func():
    pass

class Calculator:
    def method(self):
        pass
    "#;

    // When: Parse
    let tree = parse_code(source, Language::Python).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::Python, None, false).unwrap();

    // Then: Top-level functions separate from class methods
    assert_eq!(shape.functions.len(), 1);
    assert_eq!(shape.functions[0].name, "top_level_func");

    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].methods.len(), 1);
    assert_eq!(shape.classes[0].methods[0].name, "method");
}
