# Plan: Refactor treesitter-mcp Tools for LLM Coding Agents

## Goal

Redesign the treesitter-mcp toolset to be maximally useful for LLM coding agents by:
1. Returning **code context** alongside structural information
2. Adding **position-based** tools for context-aware assistance
3. Removing redundant tools
4. Creating comprehensive **multi-language test fixtures**
5. Following **RED/GREEN/BLUE** (TDD) methodology

---

## Current State

### Existing Tools
| Tool | Current Output | Problem for LLM |
|------|----------------|-----------------|
| `parse_file` | Raw S-expression AST | Too verbose, requires tree-sitter expertise |
| `file_shape` | Functions/classes/imports (names only) | Redundant with `code_map` |
| `code_map` | Directory overview (names only) | No signatures, no code |
| `find_usages` | Symbol locations + line text | No surrounding code context |
| `query_pattern` | Match locations + text | No broader code context |

### Supported Languages
- Rust (.rs)
- Python (.py)
- JavaScript (.js, .mjs, .cjs)
- TypeScript (.ts, .tsx)
- HTML (.html, .htm) - limited support
- CSS (.css) - limited support

---

## Proposed Changes

### Tools Summary

| Action | Tool | Purpose |
|--------|------|---------|
| **Modify** | `parse_file` | Return file shape (structure) instead of raw AST |
| **Modify** | `code_map` | Add detail levels, signatures, docs, code snippets |
| **Modify** | `find_usages` | Add context lines, usage type classification, code snippets |
| **Modify** | `query_pattern` | Add context lines, include code output |
| **Add** | `get_context` | Get enclosing scope at a position |
| **Add** | `get_node_at_position` | Get AST node + ancestors at position |
| **Remove** | `file_shape` | Redundant with enhanced `code_map` |

---

## Phase 1: Test Fixtures Setup

### Create Multi-Language Fixtures

Create realistic mini-projects in `tests/fixtures/` for each supported language:

```
tests/fixtures/
├── rust_project/
│   ├── src/
│   │   ├── lib.rs          # Main entry, re-exports
│   │   ├── calculator.rs   # Functions with docs
│   │   └── models/
│   │       └── mod.rs      # Structs, impls
│   └── Cargo.toml
├── python_project/
│   ├── __init__.py
│   ├── calculator.py       # Functions, classes
│   └── utils/
│       └── helpers.py      # Helper functions
├── javascript_project/
│   ├── index.js            # Main entry
│   ├── calculator.js       # ES6 functions, classes
│   └── utils/
│       └── helpers.js      # Helper functions
└── typescript_project/
    ├── index.ts            # Main entry
    ├── calculator.ts       # Typed functions, interfaces
    └── types/
        └── models.ts       # Type definitions
```

### Fixture Requirements

Each fixture must include:

1. **Functions/Methods**
   - Public and private visibility
   - With parameters and return types
   - With docstrings/comments
   - Nested (closures, inner functions)

2. **Classes/Structs**
   - With fields/properties
   - With methods
   - With doc comments

3. **Imports**
   - Internal (cross-file)
   - External (stdlib, packages)

4. **Cross-file References**
   - Function calls across files
   - Type usage across files

5. **Nested Scopes**
   - Functions inside classes
   - Closures inside functions
   - Nested classes (where applicable)

---

## Phase 2: Modify `parse_file`

### Current Behavior
Returns raw S-expression AST (tree-sitter format).

### New Behavior
Returns **file shape** - structured JSON with functions, classes, imports, and their signatures.

### New Output Format
```json
{
  "path": "src/calculator.rs",
  "language": "Rust",
  "functions": [
    {
      "name": "add",
      "signature": "pub fn add(a: i32, b: i32) -> i32",
      "line": 5,
      "end_line": 7,
      "doc": "Adds two numbers together"
    }
  ],
  "structs": [
    {
      "name": "Calculator",
      "line": 10,
      "end_line": 15,
      "doc": "A simple calculator"
    }
  ],
  "classes": [],
  "imports": [
    {"text": "use std::fmt;", "line": 1}
  ]
}
```

### Tests (RED phase)

```rust
// tests/parse_file_test.rs

#[test]
fn test_parse_file_rust_functions() {
    // Given: Rust fixture with functions
    // When: parse_file is called
    // Then: Returns JSON with function names, signatures, line numbers
}

#[test]
fn test_parse_file_rust_structs() {
    // Given: Rust fixture with structs
    // When: parse_file is called
    // Then: Returns JSON with struct names, line numbers
}

#[test]
fn test_parse_file_rust_docs() {
    // Given: Rust fixture with doc comments
    // When: parse_file is called
    // Then: Returns JSON with doc strings extracted
}

#[test]
fn test_parse_file_python_functions() {
    // Given: Python fixture with functions
    // When: parse_file is called
    // Then: Returns JSON with function names, signatures
}

#[test]
fn test_parse_file_python_classes() {
    // Given: Python fixture with classes
    // When: parse_file is called
    // Then: Returns JSON with class names, methods
}

#[test]
fn test_parse_file_javascript_functions() {
    // Given: JavaScript fixture with functions
    // When: parse_file is called
    // Then: Returns JSON with function names
}

#[test]
fn test_parse_file_javascript_classes() {
    // Given: JavaScript fixture with ES6 classes
    // When: parse_file is called
    // Then: Returns JSON with class names, methods
}

#[test]
fn test_parse_file_typescript_with_types() {
    // Given: TypeScript fixture with interfaces and typed functions
    // When: parse_file is called
    // Then: Returns JSON with types, interfaces, typed functions
}

#[test]
fn test_parse_file_nonexistent_file() {
    // Given: Path to non-existent file
    // When: parse_file is called
    // Then: Returns error
}

#[test]
fn test_parse_file_unsupported_extension() {
    // Given: File with unsupported extension
    // When: parse_file is called
    // Then: Returns error
}
```

---

## Phase 3: Modify `code_map`

### Current Behavior
Returns directory overview with function/struct/class names only.

### New Behavior
Returns directory overview with **detail levels**:
- `minimal`: Names only (current behavior)
- `signatures`: Names + full signatures
- `full`: Names + signatures + first-line docs

### New Parameters
```rust
pub struct CodeMapTool {
    pub path: String,
    pub max_tokens: Option<u32>,     // Default: 2000
    pub detail: Option<String>,       // "minimal" | "signatures" | "full", default: "signatures"
    pub pattern: Option<String>,      // Glob pattern filter, e.g., "*.rs"
}
```

### New Output Format (detail="full")
```json
{
  "files": [
    {
      "path": "src/lib.rs",
      "functions": [
        {
          "name": "parse_code",
          "signature": "pub fn parse_code(source: &str, language: Language) -> Result<Tree>",
          "line": 143,
          "doc": "Parse source code into a tree-sitter syntax tree"
        }
      ],
      "structs": [
        {
          "name": "Language",
          "line": 10,
          "doc": "Supported programming languages"
        }
      ],
      "classes": []
    }
  ],
  "truncated": false
}
```

### Tests (RED phase)

```rust
// tests/code_map_test.rs

#[test]
fn test_code_map_rust_project_minimal() {
    // Given: Rust fixture project
    // When: code_map with detail="minimal"
    // Then: Returns names only
}

#[test]
fn test_code_map_rust_project_signatures() {
    // Given: Rust fixture project
    // When: code_map with detail="signatures"
    // Then: Returns names + full signatures
}

#[test]
fn test_code_map_rust_project_full() {
    // Given: Rust fixture project
    // When: code_map with detail="full"
    // Then: Returns names + signatures + docs
}

#[test]
fn test_code_map_python_project() {
    // Given: Python fixture project
    // When: code_map is called
    // Then: Returns all Python files with structure
}

#[test]
fn test_code_map_javascript_project() {
    // Given: JavaScript fixture project
    // When: code_map is called
    // Then: Returns all JS files with structure
}

#[test]
fn test_code_map_typescript_project() {
    // Given: TypeScript fixture project
    // When: code_map is called
    // Then: Returns all TS files with structure
}

#[test]
fn test_code_map_pattern_filter() {
    // Given: Mixed language project
    // When: code_map with pattern="*.rs"
    // Then: Returns only Rust files
}

#[test]
fn test_code_map_respects_token_limit() {
    // Given: Large project
    // When: code_map with max_tokens=500
    // Then: Output is truncated, truncated=true
}

#[test]
fn test_code_map_single_file() {
    // Given: Path to single file
    // When: code_map is called
    // Then: Returns structure for that file only
}

#[test]
fn test_code_map_skips_hidden_and_vendor() {
    // Given: Project with .git, node_modules, target dirs
    // When: code_map is called
    // Then: These directories are skipped
}
```

---

## Phase 4: Modify `find_usages`

### Current Behavior
Returns file, line, column, and single line of context.

### New Behavior
Returns usage locations with:
- **Code snippets** with configurable context lines
- **Usage type** classification (definition, call, type_reference, import)
- AST node information

### New Parameters
```rust
pub struct FindUsagesTool {
    pub symbol: String,
    pub path: String,
    pub context_lines: Option<u32>,  // Default: 3
}
```

### New Output Format
```json
{
  "symbol": "parse_code",
  "usages": [
    {
      "file": "src/analysis/code_map.rs",
      "line": 164,
      "column": 29,
      "usage_type": "call",
      "node_type": "call_expression",
      "code": "    let tree = crate::parser::parse_code(&source, language).map_err(|e| {\n        io::Error::new(\n            io::ErrorKind::InvalidData,\n            format!(\"Failed to parse {} code: {e}\", language.name()),\n        )\n    })?;"
    },
    {
      "file": "src/parser/mod.rs",
      "line": 143,
      "column": 8,
      "usage_type": "definition",
      "node_type": "function_item",
      "code": "pub fn parse_code(source: &str, language: Language) -> Result<Tree> {\n    log::debug!(\"Parsing {} code ({} bytes)\", language.name(), source.len());\n    ..."
    }
  ]
}
```

### Usage Type Classification
- `definition`: Function/struct/class/variable definition
- `call`: Function/method call
- `type_reference`: Used as a type annotation
- `import`: Used in import/use statement
- `reference`: Other references (field access, etc.)

### Tests (RED phase)

```rust
// tests/find_usages_test.rs

#[test]
fn test_find_usages_rust_function_definition() {
    // Given: Rust fixture with function
    // When: find_usages for function name
    // Then: Finds definition with usage_type="definition"
}

#[test]
fn test_find_usages_rust_function_calls() {
    // Given: Rust fixture with function calls
    // When: find_usages for function name
    // Then: Finds all calls with usage_type="call"
}

#[test]
fn test_find_usages_rust_cross_file() {
    // Given: Rust fixture with cross-file references
    // When: find_usages on directory
    // Then: Finds usages in all files
}

#[test]
fn test_find_usages_rust_with_context() {
    // Given: Rust fixture
    // When: find_usages with context_lines=5
    // Then: Returns 5 lines of context around each usage
}

#[test]
fn test_find_usages_python_method() {
    // Given: Python fixture with class method
    // When: find_usages for method name
    // Then: Finds definition and calls
}

#[test]
fn test_find_usages_javascript_function() {
    // Given: JavaScript fixture
    // When: find_usages for function name
    // Then: Finds all usages
}

#[test]
fn test_find_usages_typescript_interface() {
    // Given: TypeScript fixture with interface
    // When: find_usages for interface name
    // Then: Finds definition and type references
}

#[test]
fn test_find_usages_not_found() {
    // Given: Fixture project
    // When: find_usages for non-existent symbol
    // Then: Returns empty usages array
}

#[test]
fn test_find_usages_includes_code_snippet() {
    // Given: Fixture with function usage
    // When: find_usages is called
    // Then: Each usage includes multi-line code snippet
}
```

---

## Phase 5: Modify `query_pattern`

### Current Behavior
Returns match locations with captured text.

### New Behavior
Returns matches with:
- **Code context** (configurable lines)
- **Parent node information**

### New Parameters
```rust
pub struct QueryPatternTool {
    pub file_path: String,
    pub query: String,
    pub context_lines: Option<u32>,  // Default: 2
}
```

### New Output Format
```json
{
  "query": "(function_item name: (identifier) @name)",
  "matches": [
    {
      "line": 10,
      "column": 1,
      "captures": {
        "name": "calculate"
      },
      "code": "/// Calculates the result\npub fn calculate(x: i32) -> i32 {\n    x * 2\n}",
      "parent": {
        "type": "source_file",
        "line": 1
      }
    }
  ]
}
```

### Tests (RED phase)

```rust
// tests/query_pattern_test.rs

#[test]
fn test_query_pattern_rust_functions() {
    // Given: Rust fixture
    // When: Query for function_item
    // Then: Finds all functions with code context
}

#[test]
fn test_query_pattern_rust_with_context() {
    // Given: Rust fixture
    // When: Query with context_lines=5
    // Then: Returns 5 lines of context
}

#[test]
fn test_query_pattern_python_classes() {
    // Given: Python fixture
    // When: Query for class_definition
    // Then: Finds all classes
}

#[test]
fn test_query_pattern_javascript_imports() {
    // Given: JavaScript fixture
    // When: Query for import_statement
    // Then: Finds all imports
}

#[test]
fn test_query_pattern_typescript_interfaces() {
    // Given: TypeScript fixture
    // When: Query for interface_declaration
    // Then: Finds all interfaces
}

#[test]
fn test_query_pattern_includes_parent() {
    // Given: Fixture with nested code
    // When: Query for identifier
    // Then: Result includes parent node info
}

#[test]
fn test_query_pattern_invalid_query() {
    // Given: Fixture file
    // When: Query with invalid syntax
    // Then: Returns error with helpful message
}
```

---

## Phase 6: Add `get_context`

### Purpose
Get the enclosing context (function, class, module) at a specific position.

### Parameters
```rust
pub struct GetContextTool {
    pub file_path: String,
    pub line: u32,           // 1-indexed
    pub column: Option<u32>, // 1-indexed, default: 1
}
```

### Output Format
```json
{
  "file": "src/calculator.rs",
  "position": {"line": 15, "column": 10},
  "contexts": [
    {
      "type": "function_item",
      "name": "calculate",
      "signature": "pub fn calculate(x: i32, y: i32) -> i32",
      "range": {
        "start": {"line": 10, "column": 1},
        "end": {"line": 20, "column": 2}
      },
      "code": "pub fn calculate(x: i32, y: i32) -> i32 {\n    let result = x + y;\n    // ... rest of function\n}"
    },
    {
      "type": "impl_item",
      "name": "Calculator",
      "range": {
        "start": {"line": 5, "column": 1},
        "end": {"line": 50, "column": 2}
      }
    },
    {
      "type": "source_file",
      "name": "calculator.rs"
    }
  ]
}
```

### Tests (RED phase)

```rust
// tests/get_context_test.rs

#[test]
fn test_get_context_rust_inside_function() {
    // Given: Position inside a Rust function
    // When: get_context is called
    // Then: Returns function as innermost context
}

#[test]
fn test_get_context_rust_inside_impl() {
    // Given: Position inside impl block method
    // When: get_context is called
    // Then: Returns method, then impl as contexts
}

#[test]
fn test_get_context_rust_nested_closure() {
    // Given: Position inside closure inside function
    // When: get_context is called
    // Then: Returns closure, then function as contexts
}

#[test]
fn test_get_context_python_inside_method() {
    // Given: Position inside Python class method
    // When: get_context is called
    // Then: Returns method, then class as contexts
}

#[test]
fn test_get_context_javascript_arrow_function() {
    // Given: Position inside arrow function
    // When: get_context is called
    // Then: Returns arrow function as context
}

#[test]
fn test_get_context_typescript_interface() {
    // Given: Position inside TypeScript interface
    // When: get_context is called
    // Then: Returns interface as context
}

#[test]
fn test_get_context_at_top_level() {
    // Given: Position at module top level
    // When: get_context is called
    // Then: Returns only source_file context
}

#[test]
fn test_get_context_includes_code() {
    // Given: Position inside function
    // When: get_context is called
    // Then: Innermost context includes full code
}

#[test]
fn test_get_context_invalid_position() {
    // Given: Position beyond file bounds
    // When: get_context is called
    // Then: Returns error or empty contexts
}
```

---

## Phase 7: Add `get_node_at_position`

### Purpose
Get the AST node at a specific position with ancestor chain.

### Parameters
```rust
pub struct GetNodeAtPositionTool {
    pub file_path: String,
    pub line: u32,                    // 1-indexed
    pub column: u32,                  // 1-indexed
    pub ancestor_levels: Option<u32>, // Default: 3
}
```

### Output Format
```json
{
  "file": "src/calculator.rs",
  "position": {"line": 15, "column": 10},
  "node": {
    "type": "identifier",
    "text": "result",
    "range": {
      "start": {"line": 15, "column": 9},
      "end": {"line": 15, "column": 15}
    }
  },
  "ancestors": [
    {
      "type": "let_declaration",
      "text": "let result = x + y;",
      "range": {"start": {"line": 15, "column": 5}, "end": {"line": 15, "column": 23}}
    },
    {
      "type": "block",
      "range": {"start": {"line": 11, "column": 38}, "end": {"line": 20, "column": 2}}
    },
    {
      "type": "function_item",
      "name": "calculate",
      "range": {"start": {"line": 10, "column": 1}, "end": {"line": 20, "column": 2}}
    }
  ]
}
```

### Tests (RED phase)

```rust
// tests/get_node_at_position_test.rs

#[test]
fn test_get_node_at_position_rust_identifier() {
    // Given: Position on an identifier
    // When: get_node_at_position is called
    // Then: Returns identifier node with text
}

#[test]
fn test_get_node_at_position_rust_with_ancestors() {
    // Given: Position inside nested code
    // When: get_node_at_position with ancestor_levels=5
    // Then: Returns up to 5 ancestor nodes
}

#[test]
fn test_get_node_at_position_rust_function_name() {
    // Given: Position on function name in definition
    // When: get_node_at_position is called
    // Then: Returns identifier, parent is function_item
}

#[test]
fn test_get_node_at_position_python_method_call() {
    // Given: Position on method call
    // When: get_node_at_position is called
    // Then: Returns call node with ancestors
}

#[test]
fn test_get_node_at_position_javascript_property() {
    // Given: Position on object property access
    // When: get_node_at_position is called
    // Then: Returns property node
}

#[test]
fn test_get_node_at_position_typescript_type() {
    // Given: Position on type annotation
    // When: get_node_at_position is called
    // Then: Returns type node
}

#[test]
fn test_get_node_at_position_ancestor_includes_name() {
    // Given: Position inside named construct
    // When: get_node_at_position is called
    // Then: Ancestor includes name if applicable
}

#[test]
fn test_get_node_at_position_whitespace() {
    // Given: Position on whitespace
    // When: get_node_at_position is called
    // Then: Returns nearest enclosing node or error
}
```

---

## Phase 8: Remove `file_shape`

### Changes Required

1. **Delete files**:
   - `src/analysis/file_shape.rs`
   - `tests/file_shape_tool_test.rs`

2. **Update modules**:
   - Remove `pub mod file_shape;` from `src/analysis/mod.rs`
   - Remove `FileShapeTool` from `src/tools.rs`
   - Remove from `tool_box!` macro

3. **Migrate functionality**:
   - Shape extraction logic moves into shared module used by `parse_file` and `code_map`
   - Dependency tracking (`include_deps`) is dropped (can be added later if needed)

### Tests (RED phase)

```rust
// tests/tool_registry_test.rs

#[test]
fn test_file_shape_not_registered() {
    // Given: Initialized server
    // When: tools/list is called
    // Then: file_shape is NOT in the list
}

#[test]
fn test_file_shape_call_returns_error() {
    // Given: Initialized server
    // When: tools/call with name="file_shape"
    // Then: Returns "unknown tool" error
}
```

---

## Implementation Order

Following RED/GREEN/BLUE methodology:

### Step 1: Create Test Fixtures
1. Create `tests/fixtures/` directory structure
2. Create Rust mini-project with realistic code
3. Create Python mini-project with realistic code
4. Create JavaScript mini-project with realistic code
5. Create TypeScript mini-project with realistic code

### Step 2: Write All Tests (RED)
1. Write tests for `parse_file` modifications
2. Write tests for `code_map` modifications
3. Write tests for `find_usages` modifications
4. Write tests for `query_pattern` modifications
5. Write tests for new `get_context` tool
6. Write tests for new `get_node_at_position` tool
7. Write tests for `file_shape` removal
8. Run `cargo test` - all new tests should FAIL

### Step 3: Implement Changes (GREEN)
1. Create shared shape extraction module
2. Implement `parse_file` changes
3. Implement `code_map` changes (detail levels)
4. Implement `find_usages` changes (context, usage type)
5. Implement `query_pattern` changes (context)
6. Implement `get_context` tool
7. Implement `get_node_at_position` tool
8. Remove `file_shape` tool
9. Run `cargo test` - all tests should PASS

### Step 4: Refactor (BLUE)
1. Run `cargo clippy -- -D warnings` - fix all warnings
2. Run `cargo fmt` - format all code
3. Review and optimize shared code
4. Update tool descriptions for clarity
5. Update README.md with new tool documentation

---

## File Changes Summary

### New Files
- `tests/fixtures/rust_project/src/lib.rs`
- `tests/fixtures/rust_project/src/calculator.rs`
- `tests/fixtures/rust_project/src/models/mod.rs`
- `tests/fixtures/rust_project/Cargo.toml`
- `tests/fixtures/python_project/__init__.py`
- `tests/fixtures/python_project/calculator.py`
- `tests/fixtures/python_project/utils/helpers.py`
- `tests/fixtures/javascript_project/index.js`
- `tests/fixtures/javascript_project/calculator.js`
- `tests/fixtures/javascript_project/utils/helpers.js`
- `tests/fixtures/typescript_project/index.ts`
- `tests/fixtures/typescript_project/calculator.ts`
- `tests/fixtures/typescript_project/types/models.ts`
- `src/analysis/get_context.rs`
- `src/analysis/get_node_at_position.rs`
- `src/analysis/shape.rs` (shared shape extraction)
- `tests/get_context_test.rs`
- `tests/get_node_at_position_test.rs`

### Modified Files
- `src/analysis/mod.rs` - add new modules, remove file_shape
- `src/analysis/parse_file.rs` - return file shape instead of AST
- `src/analysis/code_map.rs` - add detail levels, signatures, docs
- `src/analysis/find_usages.rs` - add context lines, usage type
- `src/analysis/query_pattern.rs` - add context lines
- `src/tools.rs` - add new tools, remove FileShapeTool
- `tests/parse_file_tool_test.rs` - update for new output
- `tests/code_map_tool_test.rs` - add detail level tests
- `tests/find_usages_tool_test.rs` - add context tests
- `tests/query_pattern_tool_test.rs` - add context tests

### Deleted Files
- `src/analysis/file_shape.rs`
- `tests/file_shape_tool_test.rs`

---

## Tree-sitter Usage Verification

All tools MUST use tree-sitter for parsing:

| Tool | Tree-sitter Usage |
|------|-------------------|
| `parse_file` | ✅ Uses `parse_code()` to get AST, then extracts shape |
| `code_map` | ✅ Uses `parse_code()` per file to extract shapes |
| `find_usages` | ✅ Uses `parse_code()` to walk AST for identifiers |
| `query_pattern` | ✅ Uses `parse_code()` + tree-sitter Query API |
| `get_context` | ✅ Uses `parse_code()` to find node at position |
| `get_node_at_position` | ✅ Uses `parse_code()` to find node at position |

---

## Success Criteria

1. ✅ All tests pass (`cargo test`)
2. ✅ No clippy warnings (`cargo clippy -- -D warnings`)
3. ✅ Code is formatted (`cargo fmt --check`)
4. ✅ All tools use tree-sitter for parsing
5. ✅ Tests cover all 4 main languages (Rust, Python, JS, TS)
6. ✅ `file_shape` tool is removed
7. ✅ New tools `get_context` and `get_node_at_position` work
8. ✅ Existing tools return code context where applicable
