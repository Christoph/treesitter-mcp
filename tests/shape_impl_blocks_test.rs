mod common;

use treesitter_mcp::analysis::shape::extract_enhanced_shape;
use treesitter_mcp::parser::{parse_code, Language};

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
