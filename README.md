# Tree-sitter MCP Server

## Overview

Tree-sitter MCP Server exposes powerful code analysis tools through the MCP protocol, allowing AI assistants to:

- Parse and analyze code structure across multiple languages
- Extract high-level file shapes without implementation details
- Generate token-aware code maps of entire projects
- Find symbol usages across codebases
- Execute custom tree-sitter queries for advanced analysis
- Analyze structural changes between file versions (diff-aware analysis)
- Identify potentially affected code when making changes

## Supported Languages

- **Rust** (.rs)
- **Python** (.py)
- **JavaScript** (.js, .mjs, .cjs)
- **TypeScript** (.ts, .tsx)
- **HTML** (.html, .htm)
- **CSS** (.css)

## Installation

### Prerequisites

- Rust toolchain (1.70 or later)
- Cargo (comes with Rust)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd treesitter-mcp

# Build release version
cargo build --release
```

## Configuration

### Adding to Claude Code CLI

```bash
claude mcp add --transport stdio treesitter-mcp -- /absolute/path/to/treesitter-mcp
```

### Adding to Gemini CLI

```bash
gemini --scope user  mcp add treesitter-mcp /absolute/path/to/treesitter-mcp
```

### Adding to Codex CLI

```bash
codex mcp add treesitter-mcp -- /absolute/path/to/treesitter-mcp
```

## Available Tools

### 1. parse_file

Parses a source file and returns the complete Abstract Syntax Tree (AST) in S-expression format.

**Use Case**: Deep structural analysis, syntax validation, understanding exact parse tree structure.

**Parameters**:
- `file_path` (string, required): Path to the source file

**Example**:
```json
{
  "file_path": "/path/to/file.rs"
}
```

**Returns**: Complete AST as S-expression string

---

### 2. file_shape

Extracts the high-level structure of a file (functions, classes, structs, imports) without implementation details.

**Use Case**: Quick overview of what's in a file, understanding module structure, finding definitions.

**Parameters**:
- `file_path` (string, required): Path to the source file
- `include_deps` (boolean, optional): Include project dependencies as a tree of nested file shapes

**Example**:
```json
{
  "file_path": "/path/to/lib.rs",
  "include_deps": true
}
```

**Returns**: JSON object with:
```json
{
  "path": "src/lib.rs",
  "functions": [
    {"name": "add", "line": 5},
    {"name": "multiply", "line": 10}
  ],
  "structs": [
    {"name": "Point", "line": 15}
  ],
  "imports": [
    "use std::fmt;"
  ],
  "dependencies": [
    {
      "path": "src/utils.rs",
      "functions": [
        {"name": "add", "line": 3}
      ],
      "imports": [],
      "dependencies": []
    }
  ]
}
```

---

### 3. code_map

Generates a high-level overview of a directory or project with token budget awareness.

**Use Case**: Understanding project structure, getting a bird's-eye view of a codebase, staying within token limits.

**Parameters**:
- `path` (string, required): Path to file or directory
- `max_tokens` (integer, optional, default: 2000): Maximum tokens for output

**Example**:
```json
{
  "path": "/path/to/project/src",
  "max_tokens": 3000
}
```

**Returns**: JSON object with aggregated file information:
```json
{
  "files": [
    {
      "path": "src/main.rs",
      "functions": ["main", "initialize"],
      "structs": ["Config"]
    },
    {
      "path": "src/parser.rs",
      "functions": ["parse_code", "detect_language"]
    }
  ],
  "truncated": false
}
```

---

### 4. find_usages

Finds all usages of a symbol (function, struct, class, variable) across files.

**Use Case**: Understanding where and how a symbol is used, refactoring, impact analysis.

**Parameters**:
- `symbol` (string, required): Symbol name to search for
- `path` (string, required): File or directory path to search in

**Example**:
```json
{
  "symbol": "helper_fn",
  "path": "/path/to/project"
}
```

**Returns**: JSON object with all usages:
```json
{
  "symbol": "helper_fn",
  "usages": [
    {
      "file": "src/main.rs",
      "line": 42,
      "column": 15,
      "context": "let result = helper_fn();"
    },
    {
      "file": "src/utils.rs",
      "line": 18,
      "column": 9,
      "context": "helper_fn() + 10"
    }
  ]
}
```

---

### 5. parse_diff

Analyzes structural changes (functions, classes added/removed/modified) between the current file and a git revision.

**Use Case**: Verify changes after editing, check if changes are cosmetic only, understand what actually changed at the symbol level (not line-by-line).

**Parameters**:
- `file_path` (string, required): Path to the source file to analyze
- `compare_to` (string, optional, default: "HEAD"): Git revision to compare against (e.g., "HEAD", "HEAD~1", "main", "abc123")

**Example**:
```json
{
  "file_path": "/path/to/calculator.rs",
  "compare_to": "HEAD"
}
```

**Returns**: JSON object with structural changes:
```json
{
  "file_path": "src/calculator.rs",
  "compare_to": "HEAD",
  "compare_to_sha": "abc123...",
  "no_structural_change": false,
  "structural_changes": [
    {
      "change_type": "signature_changed",
      "symbol_type": "function",
      "name": "add",
      "line": 15,
      "before": "fn add(a: i32, b: i32) -> i32",
      "after": "fn add(a: i64, b: i64) -> i64",
      "details": [
        {
          "kind": "parameter_changed",
          "name": "param_0",
          "from": "a: i32",
          "to": "a: i64"
        },
        {
          "kind": "return_type",
          "from": "i32",
          "to": "i64"
        }
      ]
    },
    {
      "change_type": "added",
      "symbol_type": "function",
      "name": "multiply",
      "line": 25,
      "after": "fn multiply(a: i64, b: i64) -> i64"
    }
  ],
  "summary": {
    "added": 1,
    "removed": 0,
    "modified": 1
  }
}
```

**Benefits**:
- **10-40x smaller** than re-reading entire file
- Symbol-level diff, not line-by-line
- Detects signature vs body-only changes
- Useful for verification after code generation

---

### 6. affected_by_diff

Finds all usages across the codebase that might be affected by changes in a file. Combines `parse_diff` with `find_usages` to show the "blast radius" of your changes.

**Use Case**: Understanding impact before running tests, anticipating what might break after signature changes, refactoring with confidence.

**Parameters**:
- `file_path` (string, required): Path to the changed source file
- `compare_to` (string, optional, default: "HEAD"): Git revision to compare against
- `scope` (string, optional, default: project root): Directory to search for affected usages

**Example**:
```json
{
  "file_path": "/path/to/calculator.rs",
  "compare_to": "HEAD",
  "scope": "/path/to/project"
}
```

**Returns**: JSON object with affected usages and risk levels:
```json
{
  "file_path": "src/calculator.rs",
  "compare_to": "HEAD",
  "affected_changes": [
    {
      "symbol": "add",
      "change_type": "signature_changed",
      "change_details": "fn add(a: i64, b: i64) -> i64",
      "potentially_affected": [
        {
          "file": "src/main.rs",
          "line": 42,
          "column": 15,
          "usage_type": "call",
          "code": "let sum = add(x, y);",
          "risk": "high",
          "reason": "Call site may pass wrong argument types"
        },
        {
          "file": "tests/calculator_test.rs",
          "line": 15,
          "column": 12,
          "usage_type": "call",
          "code": "assert_eq!(add(1, 2), 3);",
          "risk": "high",
          "reason": "Call site may pass wrong argument types"
        }
      ]
    }
  ],
  "summary": {
    "high_risk": 2,
    "medium_risk": 0,
    "low_risk": 0,
    "total_usages": 2
  }
}
```

**Risk Levels**:
- **High**: Signature changes affecting call sites (wrong argument count/types)
- **Medium**: Signature changes affecting type references, general symbol changes
- **Low**: Body-only changes (behavior may differ but API is same), new symbols

---

### 7. query_pattern

Executes a custom tree-sitter query pattern on a source file.

**Use Case**: Advanced code analysis, custom pattern matching, extracting specific syntax structures.

**Parameters**:
- `file_path` (string, required): Path to the source file
- `query` (string, required): Tree-sitter query in S-expression format

**Example**:
```json
{
  "file_path": "/path/to/file.rs",
  "query": "(function_item name: (identifier) @name)"
}
```

**Query Syntax Examples**:

```scheme
; Find all function names
(function_item name: (identifier) @func_name)

; Find all struct definitions
(struct_item name: (type_identifier) @struct_name)

; Find all function calls
(call_expression
  function: (identifier) @function)

; Find all imports
(use_declaration) @import
```

**Returns**: JSON object with matches:
```json
{
  "query": "(function_item name: (identifier) @name)",
  "matches": [
    {
      "line": 5,
      "column": 8,
      "text": "add",
      "captures": {
        "name": "add"
      }
    },
    {
      "line": 10,
      "column": 8,
      "text": "multiply",
      "captures": {
        "name": "multiply"
      }
    }
  ]
}
```

## Performance Considerations

- **Parsing**: Tree-sitter parsers are highly optimized and can handle large files efficiently
- **Token Limits**: The `code_map` tool respects token budgets to avoid overwhelming AI context windows
- **Caching**: Parsed trees are not cached between requests; consider using `file_shape` for repeated queries
- **Directory Traversal**: Automatically skips hidden files, `target/`, and `node_modules/`

## Contributing

Contributions are welcome! Please:

1. Follow the existing code style (use `cargo fmt`)
2. Add tests for new features (I use TDD)
3. Ensure all tests pass (`cargo test`)
4. Run clippy (`cargo clippy`)

## License

MIT

## Acknowledgments

- Built with [tree-sitter](https://tree-sitter.github.io/)
- Implements the [Model Context Protocol](https://modelcontextprotocol.io/)
- Developed using Test-Driven Development methodology
