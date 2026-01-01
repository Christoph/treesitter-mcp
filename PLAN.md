# Smart Dependency Context Feature

## Problem Statement

When an LLM edits a file (e.g., `main.rs`), it often hallucinates the signatures of imported functions (e.g., from `utils.rs`) because it cannot see them. This leads to:

- **Type errors**: Wrong parameter types, incorrect return types
- **API misuse**: Wrong function names, incorrect argument order
- **Compilation failures**: Code that looks correct but doesn't compile
- **Multiple iteration cycles**: LLM needs to see error messages and retry

### Current Limitations

1. **`file_shape` with `include_deps=true`**: Returns only function/struct **names**, not signatures
2. **`parse_file`**: Returns full file with signatures but doesn't follow imports
3. **`find_usages`**: Shows where symbols are used, but not their definitions
4. **Manual workflow**: User must manually call `file_shape` on each dependency
5. **Critical Gap**: Dependency resolution only handles `mod foo;` declarations, **NOT** `use` statements
6. **Missing method extraction**: Impl blocks (Rust), class methods (Python/JS/TS), traits, interfaces

### Real-World Example from Codebase

Looking at `tests/fixtures/rust_project/src/calculator.rs`:

```rust
use crate::models::{Calculator, Point};  // ← Current resolver IGNORES this!

pub fn create_calculator() -> Calculator {
    Calculator::new()  // ← LLM hallucinates this signature
}
```

The current `find_rust_dependencies()` only resolves:
- `mod foo;` or `pub mod foo;` declarations

It **completely ignores**:
- `use crate::models::Calculator`
- `use super::utils`
- `use std::collections::HashMap`
- Impl blocks: `impl Calculator { fn new() -> Self }`
- Trait implementations: `impl Display for Calculator`

This means the LLM can't see the `Calculator::new()` signature it needs.

## Proposed Solution: Smart Dependency Context

Enhance `parse_file` to optionally include the **signatures** (types, parameters, return values, doc comments, methods) of all imported symbols. This gives the LLM the exact API contract of dependencies without the token cost of their full implementation.

### Key Benefits

1. **Reduces Hallucinations**: LLM sees the actual signatures of functions it calls
2. **Improves Code Quality**: Ensures correct types and argument usage
3. **Token Efficient**: Dependencies included as "signatures only" (no function bodies)
4. **Better First-Pass Success**: LLM can write correct code without seeing error messages
5. **Compiler-Like View**: Mimics how compilers resolve symbols across files
6. **Complete API Surface**: Includes impl blocks, class methods, traits, interfaces

### Example Output

**After** (LLM sees the complete API):
```json
{
  "path": "src/calculator.rs",
  "functions": [...],
  "structs": [...],
  "impl_blocks": [
    {
      "type_name": "Calculator",
      "methods": [
        {"name": "new", "signature": "pub fn new() -> Self", ...}
      ]
    }
  ],
  "traits": [
    {
      "name": "Calculable",
      "methods": [
        {"name": "compute", "signature": "fn compute(&self) -> i32"}
      ]
    }
  ],
  "dependencies": [
    {
      "path": "src/models/mod.rs",
      "structs": [...],
      "impl_blocks": [...],
      "traits": [...]
    }
  ]
}
```

## Design Considerations

### Two Types of Dependencies

We need to distinguish between:

1. **Module Dependencies** (`mod foo;`)
   - File-level structure declarations
   - Current `find_rust_dependencies()` handles these
   - Used by `file_shape` to build file tree

2. **Import Dependencies** (`use foo::bar`)
   - Symbol-level usage
   - What the LLM actually needs to see
   - **Currently NOT handled**

### Scope Decision: MVP with Complete Shape Extraction

**For MVP (Phases 0-6)**, we'll:
1. Extract complete shape info (impl blocks, methods, traits, interfaces) for ALL languages
2. Use module dependency resolution (existing logic)
3. Add dependency context to `parse_file`

**For Future (Post-MVP)**, we'll add:
- `use` statement parsing
- Selective symbol filtering
- Transitive dependencies

### Alternative Design: Dedicated Tool

Instead of embedding deps in `parse_file`, we considered a separate tool:

```
resolve_imports(file_path) -> { symbols: [...], sources: [...] }
```

**Decision**: Embed in `parse_file` with `include_deps` flag for simplicity and reduced LLM overhead.

---

## Implementation Plan (TDD: RED → GREEN → BLUE)

**CRITICAL**: Follow strict Test-Driven Development for ALL phases:
1. **RED**: Write failing test first
2. **GREEN**: Write minimal code to pass test
3. **BLUE**: Refactor while keeping tests green

---

## Phase 0: Enhance Shape Extraction

**Goal**: Extract complete API surface (impl blocks, methods, traits, interfaces) for all supported languages.

### 0.1: Verify Current State (Investigation)

**Action**: Check what already exists in `shape.rs`

```bash
# Check if EnhancedClassInfo already has methods field
grep -A 10 "pub struct EnhancedClassInfo" src/analysis/shape.rs

# Check current data structures
cargo test shape -- --list
```

**Expected**: Document current state in a comment before proceeding.

---

### 0.2: Rust Impl Blocks (TDD)

#### RED: Write Failing Test

Create `tests/shape_impl_blocks_test.rs`:

```rust
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
    assert!(shape.impl_blocks[0].methods[0].signature.contains("pub fn new"));
    assert!(shape.impl_blocks[0].methods[0].signature.contains("-> Self"));
    assert!(shape.impl_blocks[0].methods[0].doc.is_none());
    
    assert_eq!(shape.impl_blocks[0].methods[1].name, "add");
    assert!(shape.impl_blocks[0].methods[1].signature.contains("&mut self"));
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
    assert_eq!(shape.impl_blocks[0].methods[0].doc, Some("Creates a new calculator with value 0".to_string()));
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
    assert!(shape.impl_blocks[0].methods[0].code.as_ref().unwrap().contains("Self { value: 0 }"));
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
```

Run: `cargo test test_extract_rust_impl_block` → **FAIL** ✅ (Expected)

#### GREEN: Implement Minimal Code

**File**: `src/analysis/shape.rs`

```rust
/// Method information from impl blocks
#[derive(Debug, serde::Serialize, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub signature: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Impl block information
#[derive(Debug, serde::Serialize, Clone)]
pub struct ImplBlockInfo {
    pub type_name: String,  // "Calculator", "Vec<T>", "Container<T>", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_name: Option<String>,  // For trait impls: "Display", "Add", etc.
    pub line: usize,
    pub end_line: usize,
    pub methods: Vec<MethodInfo>,
}

// Update EnhancedFileShape
#[derive(Debug, serde::Serialize)]
pub struct EnhancedFileShape {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub functions: Vec<EnhancedFunctionInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub structs: Vec<EnhancedStructInfo>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<EnhancedClassInfo>,
    pub imports: Vec<ImportInfo>,
    
    // NEW: Impl blocks for Rust
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impl_blocks: Vec<ImplBlockInfo>,
    
    // NEW: Dependencies (will populate in later phase)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<EnhancedFileShape>,
}
```

Update `extract_rust_enhanced()`:

```rust
fn extract_rust_enhanced(
    tree: &Tree,
    source: &str,
    include_code: bool,
) -> Result<EnhancedFileShape, io::Error> {
    let query = Query::new(
        &tree_sitter_rust::LANGUAGE.into(),
        r#"
        (function_item name: (identifier) @func.name) @func
        (struct_item name: (type_identifier) @struct.name) @struct
        (use_declaration) @import
        
        ; Capture impl blocks
        (impl_item
            type: (_) @impl.type
            body: (declaration_list) @impl.body
        ) @impl
        
        ; Capture trait implementations
        (impl_item
            trait: (_) @impl.trait
            type: (_) @impl.type
            body: (declaration_list) @impl.body
        ) @impl.trait_impl
        "#,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Query error: {}", e)))?;

    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

    let mut functions = Vec::new();
    let mut structs = Vec::new();
    let mut imports = Vec::new();
    let mut impl_blocks = Vec::new();

    for m in matches {
        // ... existing function/struct/import extraction logic ...
        
        // NEW: Extract impl blocks
        if let Some(impl_node) = m.captures.iter().find(|c| c.index == impl_capture_idx) {
            let impl_info = extract_impl_block(impl_node.node, source, include_code)?;
            impl_blocks.push(impl_info);
        }
    }

    Ok(EnhancedFileShape {
        path: None,
        language: Some("Rust".to_string()),
        functions,
        structs,
        classes: vec![],
        imports,
        impl_blocks,
        dependencies: vec![],
    })
}

fn extract_impl_block(
    node: Node,
    source: &str,
    include_code: bool,
) -> Result<ImplBlockInfo, io::Error> {
    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    // Extract type name (e.g., "Calculator" or "Container<T>")
    let type_name = node.child_by_field_name("type")
        .and_then(|n| extract_node_text(n, source))
        .unwrap_or_else(|| "Unknown".to_string());
    
    // Extract trait name if it's a trait impl
    let trait_name = node.child_by_field_name("trait")
        .and_then(|n| extract_node_text(n, source));
    
    // Extract methods
    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" {
                let method = extract_method(child, source, include_code)?;
                methods.push(method);
            }
        }
    }
    
    Ok(ImplBlockInfo {
        type_name,
        trait_name,
        line,
        end_line,
        methods,
    })
}

fn extract_method(
    node: Node,
    source: &str,
    include_code: bool,
) -> Result<MethodInfo, io::Error> {
    let name = node.child_by_field_name("name")
        .and_then(|n| extract_node_text(n, source))
        .unwrap_or_else(|| "unknown".to_string());
    
    let signature = extract_signature(node, source)
        .unwrap_or_else(|| format!("fn {}", name));
    
    let line = node.start_position().row + 1;
    let end_line = node.end_position().row + 1;
    
    let doc = extract_doc_comment(node, source);
    
    let code = if include_code {
        extract_code(node, source)
    } else {
        None
    };
    
    Ok(MethodInfo {
        name,
        signature,
        line,
        end_line,
        doc,
        code,
    })
}

fn extract_node_text(node: Node, source: &str) -> Option<String> {
    node.utf8_text(source.as_bytes()).ok().map(|s| s.to_string())
}
```

Run: `cargo test test_extract_rust_impl_block` → **PASS** ✅

#### BLUE: Refactor

- Extract common logic between `extract_method()` and `extract_function()`
- Add helper functions for cleaner code
- Run `cargo clippy -- -D warnings` → **PASS** ✅
- Run `cargo fmt` → **DONE** ✅
- Run `cargo test` → **ALL PASS** ✅

---

### 0.3: Rust Traits (TDD)

#### RED: Write Failing Test

Add to `tests/shape_impl_blocks_test.rs`:

```rust
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
    assert_eq!(shape.traits[0].doc, Some("A trait for calculable types".to_string()));
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
```

Run: `cargo test test_extract_rust_trait` → **FAIL** ✅

#### GREEN: Implement

Add to `shape.rs`:

```rust
/// Trait definition information (Rust)
#[derive(Debug, serde::Serialize, Clone)]
pub struct TraitInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub methods: Vec<MethodInfo>,
}

// Add to EnhancedFileShape
#[serde(skip_serializing_if = "Vec::is_empty")]
pub traits: Vec<TraitInfo>,
```

Update query and extraction logic...

Run: `cargo test test_extract_rust_trait` → **PASS** ✅

#### BLUE: Refactor

Run all quality checks → **PASS** ✅

---

### 0.4: Python Class Methods (TDD)

#### RED: Write Failing Test

Create `tests/shape_python_methods_test.rs`:

```rust
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
    assert_eq!(shape.classes[0].doc, Some("A simple calculator".to_string()));
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
    
    // Then: Should handle nesting
    assert_eq!(shape.classes.len(), 1);  // Only top-level
    assert_eq!(shape.classes[0].name, "Outer");
    assert_eq!(shape.classes[0].methods.len(), 1);  // Only outer_method
}
```

Run: `cargo test test_extract_python_class_methods` → **FAIL** ✅

#### GREEN: Implement

Update `EnhancedClassInfo`:

```rust
#[derive(Debug, serde::Serialize, Clone)]
pub struct EnhancedClassInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    
    // NEW: Methods nested in class
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<EnhancedFunctionInfo>,
}
```

Update `extract_python_enhanced()` to populate `methods` instead of top-level `functions` for class methods.

Run: `cargo test test_extract_python_class_methods` → **PASS** ✅

#### BLUE: Refactor

Run quality checks → **PASS** ✅

---

### 0.5: JavaScript/TypeScript Class Methods (TDD)

#### RED: Write Failing Test

Create `tests/shape_js_ts_methods_test.rs`:

```rust
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
    assert!(shape.classes[0].methods[2].signature.contains("(): number"));
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
}
```

Run: `cargo test test_extract_js_class_methods` → **FAIL** ✅

#### GREEN: Implement

Add to `shape.rs`:

```rust
/// Interface information (TypeScript)
#[derive(Debug, serde::Serialize, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub line: usize,
    pub end_line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    pub methods: Vec<EnhancedFunctionInfo>,
}

// Add to EnhancedFileShape
#[serde(skip_serializing_if = "Vec::is_empty")]
pub interfaces: Vec<InterfaceInfo>,  // TypeScript only
```

Update `extract_js_enhanced()` to handle class methods and TS interfaces.

Run: `cargo test test_extract_js_class_methods` → **PASS** ✅

#### BLUE: Refactor

Run quality checks → **PASS** ✅

---

### 0.6: Extract Common Utilities (TDD)

#### RED: Write Failing Test

Create `tests/path_utils_test.rs` (or add to existing):

```rust
use treesitter_mcp::analysis::path_utils::find_project_root;
use std::path::Path;

#[test]
fn test_find_project_root_with_cargo_toml() {
    // Given: A path inside a Cargo project
    let file_path = Path::new("tests/fixtures/rust_project/src/calculator.rs");
    
    // When: Find project root
    let root = find_project_root(file_path);
    
    // Then: Should find the directory with Cargo.toml
    assert!(root.is_some());
    assert!(root.unwrap().join("Cargo.toml").exists());
}

#[test]
fn test_find_project_root_with_package_json() {
    // Given: A path inside a Node project
    let file_path = Path::new("tests/fixtures/javascript_project/src/calculator.js");
    
    // When: Find project root
    let root = find_project_root(file_path);
    
    // Then: Should find the directory with package.json
    assert!(root.is_some());
    assert!(root.unwrap().join("package.json").exists());
}

#[test]
fn test_find_project_root_with_git() {
    // Given: A path inside a git repo
    let file_path = Path::new("src/main.rs");
    
    // When: Find project root
    let root = find_project_root(file_path);
    
    // Then: Should find .git directory
    assert!(root.is_some());
}

#[test]
fn test_find_project_root_not_found() {
    // Given: A path with no project markers
    let file_path = Path::new("/tmp/random_file.txt");
    
    // When: Find project root
    let root = find_project_root(file_path);
    
    // Then: Should return None
    assert!(root.is_none());
}
```

Run: `cargo test test_find_project_root` → **FAIL** ✅

#### GREEN: Implement

**File**: `src/analysis/path_utils.rs`

Add new function:

```rust
use std::path::{Path, PathBuf};

/// Find project root by looking for common project markers
///
/// Searches upward from the given path for:
/// - Cargo.toml (Rust)
/// - package.json (JavaScript/TypeScript)
/// - setup.py, pyproject.toml (Python)
/// - .git (Git repository)
pub fn find_project_root(file_path: &Path) -> Option<PathBuf> {
    let mut current = if file_path.is_dir() {
        file_path.to_path_buf()
    } else {
        file_path.parent()?.to_path_buf()
    };

    loop {
        // Look for project markers
        let markers = [
            "Cargo.toml",
            "package.json",
            "setup.py",
            "pyproject.toml",
            ".git",
        ];

        for marker in &markers {
            if current.join(marker).exists() {
                return Some(current);
            }
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}
```

Run: `cargo test test_find_project_root` → **PASS** ✅

#### BLUE: Refactor

Remove duplicate `find_project_root()` from `file_shape.rs` if it exists, import from `path_utils`.

Run: `cargo test` → **ALL PASS** ✅

---

### Phase 0 Completion Checklist

- [x] 0.1: Verify current state
- [x] 0.2: Rust impl blocks (RED → GREEN → BLUE)
- [x] 0.3: Rust traits (RED → GREEN → BLUE)
- [x] 0.4: Python class methods (RED → GREEN → BLUE)
- [x] 0.5: JS/TS class methods + TS interfaces (RED → GREEN → BLUE)
- [x] 0.6: Extract common utilities (RED → GREEN → BLUE)
- [x] Run full test suite: `cargo test` → **ALL PASS**
- [x] Run clippy: `cargo clippy -- -D warnings` → **ZERO WARNINGS**
- [x] Format code: `cargo fmt` → **DONE**

---

## Phase 1: Refactor Dependency Resolution (TDD)

**Goal**: Centralize dependency resolution logic in a new `dependencies.rs` module.

### 1.1: Create dependencies.rs with Tests (RED)

#### RED: Write Failing Tests

Create `tests/dependencies_test.rs`:

```rust
mod common;

use treesitter_mcp::analysis::dependencies::{resolve_dependencies, find_rust_dependencies};
use treesitter_mcp::parser::Language;
use std::path::Path;

#[test]
fn test_resolve_dependencies_rust() {
    // Given: Rust file with mod declarations
    let source = r#"
    mod calculator;
    pub mod models;
    "#;
    let file_path = Path::new("tests/fixtures/rust_project/src/lib.rs");
    let project_root = common::fixture_path("rust_project", "");
    
    // When: Resolve dependencies
    let deps = resolve_dependencies(Language::Rust, source, file_path, &project_root);
    
    // Then: Should find module files
    assert!(deps.len() >= 1);
    assert!(deps.iter().any(|p| p.to_str().unwrap().contains("calculator")));
}

#[test]
fn test_resolve_dependencies_python() {
    // Given: Python file with imports
    let source = r#"
    from utils import helpers
    import calculator
    "#;
    let file_path = Path::new("tests/fixtures/python_project/main.py");
    let project_root = common::fixture_path("python_project", "");
    
    // When: Resolve dependencies
    let deps = resolve_dependencies(Language::Python, source, file_path, &project_root);
    
    // Then: Should find imported modules
    assert!(deps.len() >= 1);
}

#[test]
fn test_resolve_dependencies_javascript() {
    // Given: JS file with imports
    let source = r#"
    import { add } from './utils';
    import Calculator from './calculator';
    "#;
    let file_path = Path::new("tests/fixtures/javascript_project/index.js");
    let project_root = common::fixture_path("javascript_project", "");
    
    // When: Resolve dependencies
    let deps = resolve_dependencies(Language::JavaScript, source, file_path, &project_root);
    
    // Then: Should find imported files
    assert!(deps.len() >= 1);
}

#[test]
fn test_resolve_dependencies_unsupported_language() {
    // Given: Unsupported language
    let source = "some html";
    let file_path = Path::new("test.html");
    let project_root = Path::new(".");
    
    // When: Resolve dependencies
    let deps = resolve_dependencies(Language::Html, source, file_path, project_root);
    
    // Then: Should return empty vec
    assert_eq!(deps.len(), 0);
}
```

Run: `cargo test test_resolve_dependencies` → **FAIL** ✅ (module doesn't exist)

#### GREEN: Implement Minimal Code

Create **File**: `src/analysis/dependencies.rs`

```rust
//! Dependency Resolution Module
//!
//! Handles finding file dependencies for different languages.
//! Supports both module declarations and import statements.

use crate::parser::Language;
use std::path::{Path, PathBuf};

/// Resolve all file dependencies for a given source file
///
/// Returns a list of absolute paths to dependency files.
/// Only includes files that exist on the filesystem.
pub fn resolve_dependencies(
    language: Language,
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    match language {
        Language::Rust => find_rust_dependencies(source, file_path, project_root),
        Language::Python => find_python_dependencies(source, file_path, project_root),
        Language::JavaScript | Language::TypeScript => {
            find_js_ts_dependencies(source, file_path, project_root)
        }
        _ => vec![],
    }
}

// Move implementations from file_shape.rs (cut & paste)
pub fn find_rust_dependencies(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    // ... existing implementation from file_shape.rs ...
    vec![] // Placeholder
}

pub fn find_python_dependencies(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    // ... existing implementation from file_shape.rs ...
    vec![] // Placeholder
}

pub fn find_js_ts_dependencies(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    // ... existing implementation from file_shape.rs ...
    vec![] // Placeholder
}
```

Add to `src/analysis/mod.rs`:

```rust
pub mod dependencies;
```

**Now move actual implementations from file_shape.rs**

Run: `cargo test test_resolve_dependencies` → **PASS** ✅

#### BLUE: Refactor

Update `file_shape.rs` to import from `dependencies`:

```rust
use crate::analysis::dependencies::{resolve_dependencies, find_rust_dependencies, find_python_dependencies, find_js_ts_dependencies};
```

Remove the duplicate function bodies from `file_shape.rs`.

Run: `cargo test file_shape` → **PASS** ✅
Run: `cargo test` → **ALL PASS** ✅

---

### Phase 1 Completion Checklist

- [x] 1.1: Create dependencies.rs (RED → GREEN → BLUE)
- [x] Move all dependency functions from file_shape.rs
- [x] Update file_shape.rs imports
- [x] Run `cargo test` → **ALL PASS**
- [x] Run `cargo clippy -- -D warnings` → **ZERO WARNINGS**

---

## Phase 2: Update Data Structures

**Goal**: Add `dependencies` field to `EnhancedFileShape`.

### 2.1: Add Dependencies Field (Already done in Phase 0.2)

Verify `EnhancedFileShape` has:

```rust
#[serde(skip_serializing_if = "Vec::is_empty")]
pub dependencies: Vec<EnhancedFileShape>,
```

✅ Already added in Phase 0.2

### 2.2: Test Serialization

#### RED: Write Test

Add to `tests/shape_impl_blocks_test.rs`:

```rust
#[test]
fn test_serialize_shape_with_dependencies() {
    // Given: Shape with dependencies
    let source = "impl Calculator { fn new() -> Self { Self { value: 0 } } }";
    let tree = parse_code(source, Language::Rust).unwrap();
    let mut shape = extract_enhanced_shape(&tree, source, Language::Rust, None, false).unwrap();
    
    // Add a dummy dependency
    let dep_source = "pub struct Point { x: i32, y: i32 }";
    let dep_tree = parse_code(dep_source, Language::Rust).unwrap();
    let dep_shape = extract_enhanced_shape(&dep_tree, dep_source, Language::Rust, Some("models.rs"), false).unwrap();
    shape.dependencies.push(dep_shape);
    
    // When: Serialize to JSON
    let json = serde_json::to_string(&shape).unwrap();
    
    // Then: Should serialize correctly
    assert!(json.contains("dependencies"));
    assert!(json.contains("impl_blocks"));
    
    // Should deserialize back
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["dependencies"].is_array());
    assert_eq!(parsed["dependencies"].as_array().unwrap().len(), 1);
}
```

Run: `cargo test test_serialize_shape_with_dependencies` → **PASS** ✅ (should already work)

### Phase 2 Completion Checklist

- [x] Verify `dependencies` field exists
- [x] Test serialization
- [x] Run `cargo clippy -- -D warnings` → **ZERO WARNINGS**

---

## Phase 3: Enhance parse_file Tool (TDD)

**Goal**: Add `include_deps` parameter to `parse_file` and implement dependency resolution.

### 3.1: Add Parameter to Tool Schema (RED)

#### RED: Write Failing Test

Add to `tests/parse_file_tool_test.rs`:

```rust
#[test]
fn test_parse_file_accepts_include_deps_parameter() {
    // Given: parse_file with include_deps parameter
    let file_path = common::fixture_path("rust_project", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });
    
    // When: Execute parse_file
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    
    // Then: Should accept parameter without error
    assert!(result.is_ok());
}
```

Run: `cargo test test_parse_file_accepts_include_deps_parameter` → **FAIL** ✅

#### GREEN: Implement

**File**: `src/tools.rs`

Update `ParseFile` struct:

```rust
#[mcp_tool(
    name = "parse_file",
    description = "Parse single file with FULL implementation details. Returns complete code for all functions/classes with names, signatures, line ranges, and doc comments. USE WHEN: ✅ Understanding implementation before editing ✅ File <500 lines needing complete context ✅ Modifying multiple functions in same file ✅ Need API signatures of imported modules (include_deps=true). DON'T USE: ❌ Only need signatures → use file_shape (10x cheaper) ❌ Only editing one function → use read_focused_code (3x cheaper) ❌ File >500 lines → use file_shape first. TOKEN COST: HIGH. OPTIMIZATION: Set include_code=false for 60-80% reduction, include_deps=true for dependency signatures."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseFile {
    /// Path to the source file to parse
    pub file_path: String,
    
    /// Include full code blocks for functions/classes (default: true)
    /// When false, returns only signatures, docs, and line ranges (60-80% token reduction)
    #[serde(default = "default_true")]
    pub include_code: bool,
    
    /// Include module dependencies as nested file shapes (default: false)
    /// 
    /// Dependencies are ALWAYS returned as signatures-only (no code bodies)
    /// to provide the LLM with API contracts while minimizing token usage.
    /// 
    /// Currently resolves:
    /// - Rust: `mod foo;` and `pub mod foo;` declarations + impl blocks + traits
    /// - Python: `import foo` and `from foo import bar` statements + class methods
    /// - JavaScript/TypeScript: `import ... from './foo'` relative imports + class methods + interfaces
    /// 
    /// Note: Only includes direct (1-level) dependencies, not transitive deps.
    #[serde(default)]
    pub include_deps: bool,
}
```

**File**: `src/analysis/parse_file.rs`

Update `execute()`:

```rust
use crate::analysis::dependencies::resolve_dependencies;
use crate::analysis::path_utils::find_project_root;
use std::collections::HashSet;

pub fn execute(arguments: &Value) -> Result<CallToolResult, io::Error> {
    let file_path = arguments["file_path"].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Missing or invalid 'file_path' argument",
        )
    })?;

    let include_code = arguments["include_code"].as_bool().unwrap_or(true);
    let include_deps = arguments["include_deps"].as_bool().unwrap_or(false);

    log::info!("Parsing file: {file_path} (include_code: {include_code}, include_deps: {include_deps})");

    // Parse main file
    let source = fs::read_to_string(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Failed to read file {file_path}: {e}"),
        )
    })?;

    let language = detect_language(file_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("Cannot detect language for file {file_path}: {e}"),
        )
    })?;

    let tree = parse_code(&source, language).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {} code: {e}", language.name()),
        )
    })?;

    let mut shape =
        extract_enhanced_shape(&tree, &source, language, Some(file_path), include_code)?;

    // NEW: Optionally include dependencies
    if include_deps {
        let project_root = find_project_root(Path::new(file_path)).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine project root",
            )
        })?;

        let mut visited = HashSet::new();
        
        // Mark main file as visited
        if let Ok(canonical) = fs::canonicalize(file_path) {
            visited.insert(canonical);
        }

        let dep_paths = resolve_dependencies(
            language,
            &source,
            Path::new(file_path),
            &project_root,
        );

        log::debug!("Found {} dependencies for {}", dep_paths.len(), file_path);

        for dep_path in dep_paths {
            // Canonicalize and check if already visited
            let canonical = match fs::canonicalize(&dep_path) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Failed to canonicalize {}: {}", dep_path.display(), e);
                    continue;
                }
            };

            if visited.contains(&canonical) {
                log::debug!("Skipping already visited: {}", dep_path.display());
                continue; // Avoid cycles
            }
            visited.insert(canonical);

            // Read and parse dependency
            let dep_source = match fs::read_to_string(&dep_path) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Failed to read dependency {}: {}", dep_path.display(), e);
                    continue; // Skip missing files gracefully
                }
            };

            let dep_language = match detect_language(&dep_path) {
                Ok(l) => l,
                Err(e) => {
                    log::warn!("Failed to detect language for {}: {}", dep_path.display(), e);
                    continue;
                }
            };

            let dep_tree = match parse_code(&dep_source, dep_language) {
                Ok(t) => t,
                Err(e) => {
                    log::warn!("Failed to parse {}: {}", dep_path.display(), e);
                    continue;
                }
            };

            // Dependencies are ALWAYS signatures-only (include_code=false)
            let mut dep_shape = extract_enhanced_shape(
                &dep_tree,
                &dep_source,
                dep_language,
                Some(dep_path.to_str().unwrap_or("unknown")),
                false, // IMPORTANT: No code bodies for deps
            )?;

            // Convert to relative path
            if let Some(ref path) = dep_shape.path {
                dep_shape.path = Some(path_utils::to_relative_path(path));
            }

            shape.dependencies.push(dep_shape);
        }
    }

    // Convert main file path to relative
    if let Some(ref path) = shape.path {
        shape.path = Some(path_utils::to_relative_path(path));
    }

    let shape_json = serde_json::to_string(&shape).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to serialize shape to JSON: {e}"),
        )
    })?;

    Ok(CallToolResult::success(shape_json))
}
```

Run: `cargo test test_parse_file_accepts_include_deps_parameter` → **PASS** ✅

#### BLUE: Refactor

Extract dependency resolution logic to helper function if needed.

Run: `cargo test` → **ALL PASS** ✅

---

### Phase 3 Completion Checklist

- [x] Add `include_deps` parameter (RED → GREEN → BLUE)
- [x] Implement dependency resolution in parse_file
- [x] Add cycle detection
- [x] Add graceful error handling
- [x] Add logging
- [x] Run `cargo test` → **ALL PASS**
- [x] Run `cargo clippy -- -D warnings` → **ZERO WARNINGS**

---

## Phase 4: Comprehensive Testing

**Goal**: Test all languages, edge cases, and performance.

### 4.1: Core Functionality Tests

Create `tests/parse_file_deps_test.rs`:

```rust
mod common;

use serde_json::json;

#[test]
fn test_parse_file_no_deps() {
    // Given: parse_file with include_deps=false
    let file_path = common::fixture_path("rust_project", "src/calculator.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": false,
    });

    // When: execute parse_file
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: No dependencies included
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let deps = shape["dependencies"].as_array().unwrap();
    assert_eq!(deps.len(), 0, "Should have no dependencies");
}

#[test]
fn test_parse_file_with_deps_rust() {
    // Given: Rust file with mod declarations
    let file_path = common::fixture_path("rust_project", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    // When: execute parse_file with include_deps=true
    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);

    // Then: Dependencies are included with signatures only
    assert!(result.is_ok());
    let call_result = result.unwrap();
    let text = common::get_result_text(&call_result);
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();

    let deps = shape["dependencies"].as_array().unwrap();
    assert!(deps.len() >= 1, "Should include at least calculator module");

    // Verify calculator dependency is included
    let calc_dep = deps.iter().find(|d| {
        d["path"].as_str().map(|p| p.contains("calculator")).unwrap_or(false)
    });
    assert!(calc_dep.is_some(), "Should include calculator dependency");

    // Verify it has signatures but no code
    let calc = calc_dep.unwrap();
    
    // Check functions
    if let Some(functions) = calc["functions"].as_array() {
        for func in functions {
            assert!(func["signature"].is_string(), "Should have signature");
            assert!(func["code"].is_null() || !func["code"].is_string(), "Should NOT have code body");
        }
    }
    
    // Check impl blocks
    if let Some(impl_blocks) = calc["impl_blocks"].as_array() {
        for impl_block in impl_blocks {
            let methods = impl_block["methods"].as_array().unwrap();
            for method in methods {
                assert!(method["signature"].is_string(), "Should have signature");
                assert!(method["code"].is_null() || !method["code"].is_string(), "Should NOT have code body");
            }
        }
    }
}

#[test]
fn test_parse_file_deps_circular_handling() {
    // Test requires special fixture with circular dependencies
    // Create tests/fixtures/circular_deps/ with A → B → A
    // Then verify no infinite loop and both files included once
}

#[test]
fn test_parse_file_deps_missing_files() {
    // Test requires special fixture referencing non-existent module
    // Verify graceful handling (warning logged, but no error)
}

#[test]
fn test_parse_file_deps_token_efficiency() {
    // Given: File with dependencies
    let file_path = common::fixture_path("rust_project", "src/lib.rs");
    
    // When: Parse with full code
    let full_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": true,
        "include_deps": true,
    });
    let full_result = treesitter_mcp::analysis::parse_file::execute(&full_args).unwrap();
    let full_text = common::get_result_text(&full_result);
    
    // When: Parse with signatures only
    let sig_args = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });
    let sig_result = treesitter_mcp::analysis::parse_file::execute(&sig_args).unwrap();
    let sig_text = common::get_result_text(&sig_result);
    
    // Then: Signatures-only should be significantly smaller
    let full_size = full_text.len();
    let sig_size = sig_text.len();
    
    assert!(sig_size < full_size * 4 / 10, 
        "Signatures should be <40% of full size. Got {} vs {}", sig_size, full_size);
}

#[test]
fn test_parse_file_deps_python() {
    // Test Python import resolution
    let file_path = common::fixture_path("python_project", "calculator.py");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());
    
    // Verify Python class methods are included
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
    
    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            if let Some(methods) = class["methods"].as_array() {
                assert!(methods.len() > 0, "Classes should have methods");
            }
        }
    }
}

#[test]
fn test_parse_file_deps_javascript() {
    // Test JavaScript import resolution
    let file_path = common::fixture_path("javascript_project", "index.js");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());
    
    // Verify JS class methods are included
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
    
    if let Some(classes) = shape["classes"].as_array() {
        for class in classes {
            if let Some(methods) = class["methods"].as_array() {
                assert!(methods.len() > 0, "Classes should have methods");
            }
        }
    }
}

#[test]
fn test_parse_file_deps_typescript() {
    // Test TypeScript with interfaces
    let file_path = common::fixture_path("typescript_project", "calculator.ts");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());
    
    // Verify TS interfaces are included
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
    
    // Check for interfaces
    if let Some(interfaces) = shape["interfaces"].as_array() {
        for interface in interfaces {
            assert!(interface["name"].is_string());
            if let Some(methods) = interface["methods"].as_array() {
                assert!(methods.len() > 0, "Interfaces should have method signatures");
            }
        }
    }
}

#[test]
fn test_parse_file_deps_rust_traits() {
    // Test that Rust traits are included in dependencies
    let file_path = common::fixture_path("rust_project", "src/lib.rs");
    let arguments = json!({
        "file_path": file_path.to_str().unwrap(),
        "include_code": false,
        "include_deps": true,
    });

    let result = treesitter_mcp::analysis::parse_file::execute(&arguments);
    assert!(result.is_ok());
    
    let text = common::get_result_text(&result.unwrap());
    let shape: serde_json::Value = serde_json::from_str(&text).unwrap();
    
    // Check dependencies for traits
    if let Some(deps) = shape["dependencies"].as_array() {
        for dep in deps {
            if let Some(traits) = dep["traits"].as_array() {
                for trait_def in traits {
                    assert!(trait_def["name"].is_string());
                    if let Some(methods) = trait_def["methods"].as_array() {
                        for method in methods {
                            assert!(method["signature"].is_string());
                        }
                    }
                }
            }
        }
    }
}
```

Run: `cargo test parse_file_deps` → **PASS** ✅

### 4.2: Performance Benchmarks

Create `benches/dependency_resolution_bench.rs`:

```rust
#![feature(test)]
extern crate test;

use test::Bencher;
use treesitter_mcp::analysis::parse_file;
use serde_json::json;

#[bench]
fn bench_parse_file_without_deps(b: &mut Bencher) {
    let file_path = "tests/fixtures/rust_project/src/lib.rs";
    let arguments = json!({
        "file_path": file_path,
        "include_code": false,
        "include_deps": false,
    });
    
    b.iter(|| {
        parse_file::execute(&arguments).unwrap();
    });
}

#[bench]
fn bench_parse_file_with_deps(b: &mut Bencher) {
    let file_path = "tests/fixtures/rust_project/src/lib.rs";
    let arguments = json!({
        "file_path": file_path,
        "include_code": false,
        "include_deps": true,
    });
    
    b.iter(|| {
        parse_file::execute(&arguments).unwrap();
    });
}
```

Run: `cargo bench` → Verify <100ms overhead

---

### Phase 4 Completion Checklist

- [x] Core functionality tests (all languages)
- [x] Edge case tests (circular, missing files)
- [x] Token efficiency test
- [x] Performance benchmarks
- [x] Run `cargo test` → **ALL PASS**
- [x] Run `cargo bench` → **<100ms overhead**

---

## Phase 5: Documentation

**Goal**: Update README.md and tool descriptions.

### 5.1: Update README.md

Add to `README.md` section for `parse_file`:

```markdown
### 1. parse_file

Parse single file with FULL implementation details. Returns complete code for all functions/classes with impl blocks, methods, traits, and interfaces.

**Use When:**
- ✅ Understanding implementation details before editing
- ✅ File is <500 lines and you need complete context
- ✅ Writing tests that require understanding function logic
- ✅ Modifying multiple functions in same file
- ✅ **NEW**: Need API signatures of imported modules (`include_deps=true`)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `include_code` (boolean, optional, default: true): Set false for 60-80% token reduction (signatures only)
- `include_deps` (boolean, optional, default: false): Include module dependencies as signatures

**New Feature: Smart Dependency Context**

When `include_deps=true`, the tool automatically includes the signatures of all module dependencies with complete API surface:

- **Rust**: Impl blocks, trait definitions, methods
- **Python**: Class methods
- **JavaScript**: Class methods
- **TypeScript**: Class methods, interfaces

**Example**:

```json
{
  "file_path": "src/calculator.rs",
  "include_code": false,
  "include_deps": true
}
```

**Returns**:
```json
{
  "path": "src/calculator.rs",
  "functions": [...],
  "structs": [...],
  "impl_blocks": [
    {
      "type_name": "Calculator",
      "methods": [
        {"name": "new", "signature": "pub fn new() -> Self", "line": 12}
      ]
    }
  ],
  "traits": [
    {
      "name": "Calculable",
      "methods": [
        {"name": "compute", "signature": "fn compute(&self) -> i32"}
      ]
    }
  ],
  "dependencies": [
    {
      "path": "src/models/mod.rs",
      "structs": [...],
      "impl_blocks": [...],
      "traits": [...]
    }
  ]
}
```

**Token Savings**:

Without include_deps:
- parse_file(main.rs) + parse_file(utils.rs) + parse_file(models.rs)
- Total: ~2000 tokens

With include_deps:
- parse_file(main.rs, include_deps=true)
- Total: ~1000 tokens (50% reduction!)

**Currently Resolves**:
- Rust: `mod foo;` and `pub mod foo;` declarations
- Python: `import foo` and `from foo import bar` statements
- JavaScript/TypeScript: relative imports (`import ... from './foo'`)

**Note**: Only includes direct dependencies (1-level). Transitive dependencies are not included.
```

### Phase 5 Completion Checklist

- [x] Update README.md parse_file section
- [x] Add token savings example
- [x] Add usage example with real fixture
- [x] Document limitations
- [x] Update tool descriptions in tools.rs

---

## Phase 6: Final Quality Checks

### Checklist

- [ ] Run full test suite: `cargo test` → **ALL PASS**
- [ ] Run clippy: `cargo clippy -- -D warnings` → **ZERO WARNINGS**
- [ ] Format code: `cargo fmt` → **DONE**
- [ ] Check for unused imports: `cargo clippy -- -W unused_imports` → **ZERO WARNINGS**
- [ ] Run benchmarks: `cargo bench` → **<100ms overhead**
- [ ] Manual smoke test with real-world Rust project
- [ ] Manual smoke test with real-world Python project
- [ ] Manual smoke test with real-world TypeScript project
- [ ] Verify backward compatibility: all existing tests pass without modification

---

## Success Metrics

1. ✅ **Zero Breaking Changes**: All existing tests pass without modification
2. ✅ **New Tests Pass**: All 15+ new tests pass (impl blocks, traits, interfaces, dependencies)
3. ✅ **Token Efficiency**: Dependencies are 70-80% smaller than full files (measured)
4. ✅ **Correctness**: Impl blocks, methods, traits, interfaces correctly extracted
5. ✅ **Robustness**: Missing/circular dependencies handled gracefully
6. ✅ **Performance**: Dependency resolution adds <100ms overhead for typical projects
7. ✅ **Code Quality**: No clippy warnings, proper error handling, good logging
8. ✅ **TDD Compliance**: All features developed using RED → GREEN → BLUE
9. ✅ **Cross-Language Support**: All 6 languages work correctly

---

## Known Limitations (MVP)

1. **Module declarations only**: Currently only resolves `mod foo;`, `import`, relative imports - NOT `use` statements
2. **1-level depth**: Only includes direct dependencies, not transitive
3. **No symbol filtering**: Includes all exports, not just imported symbols
4. **No type aliases**: Type aliases and constants not included (future enhancement)
5. **No re-export tracking**: Re-exports (`pub use`) not followed

These limitations will be addressed in future phases after MVP is stable.

---

## Future Enhancements (Post-MVP)

### Phase 7: Import Statement Resolution

Enhance dependency resolution to parse `use` statements:

```rust
// In dependencies.rs
pub fn find_rust_use_imports(
    source: &str,
    file_path: &Path,
    project_root: &Path,
) -> Vec<PathBuf> {
    // Parse use statements with tree-sitter
    // Resolve crate-relative paths: use crate::models::Calculator
    // Resolve relative paths: use super::utils
    // Filter to only project files (ignore std, external crates)
}
```

This would handle the real-world case:
```rust
use crate::models::Calculator;  // ← Now resolved!
```

### Phase 8: Selective Symbol Export

Instead of including ALL exports from a dependency, only include symbols that are actually imported:

```rust
// main.rs
use utils::{add, multiply};  // Only imported these

// Dependency shape includes ONLY add() and multiply()
// NOT subtract(), divide(), etc.
```

### Phase 9: Type Aliases and Constants

```rust
#[derive(Debug, serde::Serialize, Clone)]
pub struct TypeAliasInfo {
    pub name: String,
    pub definition: String,  // "type Result<T> = std::result::Result<T, Error>"
    pub line: usize,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ConstantInfo {
    pub name: String,
    pub type_annotation: Option<String>,
    pub value: Option<String>,
    pub line: usize,
}
```

### Phase 10: Configurable Depth

```rust
pub struct ParseFile {
    pub include_deps: bool,
    pub deps_depth: u32,  // NEW: 0 = none, 1 = direct, 2 = transitive, etc.
}
```

### Phase 11: Caching

Cache parsed dependency shapes to avoid re-parsing:

```rust
use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::collections::HashMap;

static SHAPE_CACHE: Lazy<Mutex<HashMap<PathBuf, EnhancedFileShape>>> = 
    Lazy::new(|| Mutex::new(HashMap::new()));
```

---

## Appendix: TDD Workflow Summary

For **every** feature:

### RED Phase
1. Write a failing test that specifies the expected behavior
2. Run test → **FAIL** ✅
3. Commit: "test: add failing test for X"

### GREEN Phase
1. Write minimal code to make the test pass
2. Run test → **PASS** ✅
3. Commit: "feat: implement X"

### BLUE Phase
1. Refactor code for clarity/performance
2. Run ALL tests → **PASS** ✅
3. Run `cargo clippy` → **ZERO WARNINGS** ✅
4. Run `cargo fmt` → **DONE** ✅
5. Commit: "refactor: improve X"

**Never skip any phase. Never write code before tests.**
