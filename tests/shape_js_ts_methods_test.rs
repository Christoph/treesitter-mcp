mod common;

use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{parse_code, Language};

#[test]
fn test_extract_js_class_methods() {
    // Given: JavaScript class
    let source = r#"
    class Calculator {
        constructor(value = 0) {
            this.value = value;
        }
        
        add(x) {
            this.value += x;
        }
        
        getValue() {
            return this.value;
        }
    }
    "#;

    // When: Parse
    let tree = parse_code(source, Language::JavaScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

    // Then: Should have class with methods
    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].name, "Calculator");
    assert_eq!(shape.classes[0].methods.len(), 3);

    assert_eq!(shape.classes[0].methods[0].name, "constructor");
    assert_eq!(shape.classes[0].methods[1].name, "add");
    assert_eq!(shape.classes[0].methods[2].name, "getValue");
}

#[test]
fn test_extract_ts_class_methods_with_types() {
    // Given: TypeScript class with type annotations
    let source = r#"
    class Calculator {
        private value: number;
        
        constructor(value: number = 0) {
            this.value = value;
        }
        
        add(x: number): void {
            this.value += x;
        }
        
        getValue(): number {
            return this.value;
        }
    }
    "#;

    // When: Parse
    let tree = parse_code(source, Language::TypeScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

    // Then: Should capture type annotations
    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].methods.len(), 3);

    assert!(shape.classes[0].methods[1].signature.contains("number"));
    assert!(shape.classes[0].methods[2].signature.contains("number"));
}

#[test]
fn test_extract_ts_interface() {
    // Given: TypeScript interface
    let source = r#"
    interface Calculable {
        compute(): number;
        reset(): void;
    }
    "#;

    // When: Parse
    let tree = parse_code(source, Language::TypeScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

    // Then: Should have interface definition
    assert_eq!(shape.interfaces.len(), 1);
    assert_eq!(shape.interfaces[0].name, "Calculable");
    assert_eq!(shape.interfaces[0].methods.len(), 2);

    assert_eq!(shape.interfaces[0].methods[0].name, "compute");
    assert_eq!(shape.interfaces[0].methods[1].name, "reset");
}

#[test]
fn test_extract_js_class_with_code() {
    // Given: JavaScript class
    let source = r#"
    class Calculator {
        add(x) {
            this.value += x;
        }
    }
    "#;

    // When: Parse with include_code=true
    let tree = parse_code(source, Language::JavaScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::JavaScript, None, true).unwrap();

    // Then: Should include method code
    assert!(shape.classes[0].methods[0].code.is_some());
    assert!(shape.classes[0].methods[0]
        .code
        .as_ref()
        .unwrap()
        .contains("this.value += x"));
}

#[test]
fn test_extract_js_class_without_code() {
    // Given: JavaScript class
    let source = r#"
    class Calculator {
        add(x) {
            this.value += x;
        }
    }
    "#;

    // When: Parse with include_code=false
    let tree = parse_code(source, Language::JavaScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

    // Then: Should NOT include method code
    assert!(shape.classes[0].methods[0].code.is_none());
}

#[test]
fn test_extract_ts_interface_with_optional_methods() {
    // Given: TypeScript interface with optional methods
    let source = r#"
    interface Calculator {
        add(x: number): void;
        subtract?(x: number): void;
    }
    "#;

    // When: Parse
    let tree = parse_code(source, Language::TypeScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

    // Then: Should capture both methods
    assert_eq!(shape.interfaces.len(), 1);
    assert_eq!(shape.interfaces[0].methods.len(), 2);
}

#[test]
fn test_extract_js_top_level_functions_separate() {
    // Given: Mix of top-level functions and class methods
    let source = r#"
    function topLevelFunc() {
        return 42;
    }
    
    class Calculator {
        method() {
            return 0;
        }
    }
    "#;

    // When: Parse
    let tree = parse_code(source, Language::JavaScript).unwrap();
    let shape = extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

    // Then: Top-level functions separate from class methods
    assert_eq!(shape.functions.len(), 1);
    assert_eq!(shape.functions[0].name, "topLevelFunc");

    assert_eq!(shape.classes.len(), 1);
    assert_eq!(shape.classes[0].methods.len(), 1);
    assert_eq!(shape.classes[0].methods[0].name, "method");
}
