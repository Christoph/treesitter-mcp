//! Unit tests for shape extraction internals
//!
//! These tests verify the implementation details of extract_enhanced_shape.
//! For tool-level behavior tests, see integration tests in tests/ directory.

#[cfg(test)]
mod tests {
    use crate::analysis::shape::extract_enhanced_shape;
    use crate::parser::{parse_code, Language};

    // ========================================================================
    // Rust Impl Blocks (from shape_impl_blocks_test.rs)
    // ========================================================================

    #[test]
    fn test_extract_rust_impl_block_basic() {
        // Given: Rust code with impl block
        let source = r#"
    pub struct Calculator {
        value: i32,
    }
    
    impl Calculator {
        pub fn new() -> Self {
            Self { value: 0 }
        }
        
        pub fn add(&mut self, x: i32) {
            self.value += x;
        }
    }
    "#;

        // When: Parse and extract shape
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should have impl block with methods
        assert_eq!(shape.impl_blocks.len(), 1, "Should have 1 impl block");
        assert_eq!(shape.impl_blocks[0].type_name, "Calculator");
        assert_eq!(shape.impl_blocks[0].trait_name, None);
        assert_eq!(shape.impl_blocks[0].methods.len(), 2);

        // Verify method details
        assert_eq!(shape.impl_blocks[0].methods[0].name, "new");
        assert!(shape.impl_blocks[0].methods[0]
            .signature
            .contains("pub fn new"));
        assert!(shape.impl_blocks[0].methods[0]
            .signature
            .contains("-> Self"));
        assert!(shape.impl_blocks[0].methods[0].doc.is_none());

        assert_eq!(shape.impl_blocks[0].methods[1].name, "add");
        assert!(shape.impl_blocks[0].methods[1]
            .signature
            .contains("&mut self"));
    }

    #[test]
    fn test_extract_rust_trait_impl_block() {
        // Given: Trait implementation
        let source = r#"
    use std::fmt;
    
    pub struct Calculator {
        value: i32,
    }
    
    impl fmt::Display for Calculator {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.value)
        }
    }
    "#;

        // When: Parse and extract shape
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should capture trait name
        assert_eq!(shape.impl_blocks.len(), 1);
        assert_eq!(shape.impl_blocks[0].type_name, "Calculator");
        assert_eq!(shape.impl_blocks[0].trait_name, Some("Display".to_string()));
        assert_eq!(shape.impl_blocks[0].methods.len(), 1);
        assert_eq!(shape.impl_blocks[0].methods[0].name, "fmt");
    }

    #[test]
    fn test_extract_rust_impl_block_with_docs() {
        // Given: Impl block with doc comments
        let source = r#"
    pub struct Calculator {
        value: i32,
    }
    
    impl Calculator {
        /// Creates a new calculator with value 0
        pub fn new() -> Self {
            Self { value: 0 }
        }
    }
    "#;

        // When: Parse and extract shape
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should capture doc comments
        assert_eq!(
            shape.impl_blocks[0].methods[0].doc,
            Some("Creates a new calculator with value 0".to_string())
        );
    }

    #[test]
    fn test_extract_rust_impl_block_include_code() {
        // Given: Parse with include_code=true
        let source = r#"
    impl Calculator {
        pub fn new() -> Self {
            Self { value: 0 }
        }
    }
    "#;

        // When: Parse with include_code=true
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, true).unwrap();

        // Then: Should include method body
        assert!(shape.impl_blocks[0].methods[0].code.is_some());
        assert!(shape.impl_blocks[0].methods[0]
            .code
            .as_ref()
            .unwrap()
            .contains("Self { value: 0 }"));
    }

    #[test]
    fn test_extract_rust_impl_block_no_code() {
        // Given: Parse with include_code=false
        let source = r#"
    impl Calculator {
        pub fn new() -> Self {
            Self { value: 0 }
        }
    }
    "#;

        // When: Parse with include_code=false
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should NOT include method body
        assert!(shape.impl_blocks[0].methods[0].code.is_none());
    }

    #[test]
    fn test_extract_rust_empty_impl_block() {
        // Given: Empty impl block
        let source = r#"
    impl Calculator {
    }
    "#;

        // When: Parse
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should handle gracefully
        assert_eq!(shape.impl_blocks.len(), 1);
        assert_eq!(shape.impl_blocks[0].methods.len(), 0);
    }

    #[test]
    fn test_extract_rust_generic_impl_block() {
        // Given: Generic impl block
        let source = r#"
    impl<T> Container<T> {
        pub fn new(value: T) -> Self {
            Self { value }
        }
    }
    "#;

        // When: Parse
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should capture generic type name
        assert_eq!(shape.impl_blocks.len(), 1);
        assert!(shape.impl_blocks[0].type_name.contains("Container"));
    }

    #[test]
    fn test_extract_rust_trait_definition() {
        // Given: Rust trait definition
        let source = r#"
    /// A trait for calculable types
    pub trait Calculable {
        /// Compute the result
        fn compute(&self) -> i32;
        
        fn reset(&mut self);
    }
    "#;

        // When: Parse and extract shape
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();

        // Then: Should have trait definition
        assert_eq!(shape.traits.len(), 1);
        assert_eq!(shape.traits[0].name, "Calculable");
        assert_eq!(
            shape.traits[0].doc,
            Some("A trait for calculable types".to_string())
        );
        assert_eq!(shape.traits[0].methods.len(), 2);

        assert_eq!(shape.traits[0].methods[0].name, "compute");
        assert!(shape.traits[0].methods[0].signature.contains("&self"));
        assert!(shape.traits[0].methods[0].signature.contains("-> i32"));

        assert_eq!(shape.traits[0].methods[1].name, "reset");
    }

    #[test]
    fn test_extract_rust_trait_with_default_impl() {
        // Given: Trait with default implementation
        let source = r#"
    pub trait Calculable {
        fn compute(&self) -> i32 {
            0
        }
    }
    "#;

        // When: Parse with include_code=true
        let tree = parse_code(source, Language::Rust).unwrap();
        let shape = extract_enhanced_shape(&tree, source, Language::Rust, None, true).unwrap();

        // Then: Should include default implementation
        assert!(shape.traits[0].methods[0].code.is_some());
    }

    // ========================================================================
    // Python Methods (from shape_python_methods_test.rs)
    // ========================================================================

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

    // ========================================================================
    // JavaScript/TypeScript Methods (from shape_js_ts_methods_test.rs)
    // ========================================================================

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::JavaScript, None, true).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::TypeScript, None, false).unwrap();

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::JavaScript, None, false).unwrap();

        // Then: Top-level functions separate from class methods
        assert_eq!(shape.functions.len(), 1);
        assert_eq!(shape.functions[0].name, "topLevelFunc");

        assert_eq!(shape.classes.len(), 1);
        assert_eq!(shape.classes[0].methods.len(), 1);
        assert_eq!(shape.classes[0].methods[0].name, "method");
    }

    // ========================================================================
    // Module/JSON Format (from shape_module_test.rs)
    // ========================================================================

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
        let shape =
            extract_enhanced_shape(&tree, source, Language::JavaScript, Some("calc.js"), true)
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
}
