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

### Quick Tool Selection Guide

Choose the right tool for your task:

#### "I need to understand code"
- **Don't know which file?** → `code_map` (directory overview)
- **Know the file, need overview?** → `file_shape` (signatures only, 10x cheaper than parse_file)
- **Know the file, need full details?** → `parse_file` (complete code)
- **Know the specific function?** → `read_focused_code` (focused view, 3x cheaper than parse_file)

#### "I need to find something"
- **Where is symbol X used?** → `find_usages` (semantic search with usage types)
- **Complex pattern matching?** → `query_pattern` (advanced, requires tree-sitter syntax)
- **What function is at line N?** → `get_context` (scope hierarchy)
- **What's the exact AST node?** → `get_node_at_position` (syntax details, advanced)

#### "I'm refactoring/changing code"
- **Before changes:** `find_usages` (see all usages)
- **After changes:** `parse_diff` (verify changes at symbol level)
- **Impact analysis:** `affected_by_diff` (what might break with risk levels)

### Tool Comparison Matrix

| Tool | Scope | Token Cost | Speed | Best For |
|------|-------|------------|-------|----------|
| `code_map` | Directory | Medium | Fast | First-time exploration |
| `file_shape` | Single file | **Low** | Fast | Quick overview, API understanding |
| `parse_file` | Single file | **High** | Fast | Deep understanding, multiple functions |
| `read_focused_code` | Single file | Medium | Fast | Editing specific function |
| `find_usages` | Multi-file | Medium-High | Medium | Refactoring, impact analysis |
| `affected_by_diff` | Multi-file | Medium-High | Medium | Post-change validation |
| `parse_diff` | Single file | **Low-Medium** | Fast | Verify changes |
| `get_context` | Single file | **Low** | Fast | Error debugging, scope lookup |
| `get_node_at_position` | Single file | **Low** | Fast | Syntax-aware edits (advanced) |
| `query_pattern` | Single file | Medium | Medium | Complex patterns (advanced) |

### Common Workflow Patterns

#### Pattern 1: Exploring New Codebase
```
1. code_map (path="src", detail="minimal")      → Get lay of the land
2. file_shape (interesting files)               → Understand interfaces
3. read_focused_code (specific functions)       → Deep dive
```

#### Pattern 2: Refactoring Function
```
1. find_usages (symbol="function_name")         → See all call sites
2. Make changes
3. parse_diff ()                                → Verify changes
4. affected_by_diff ()                          → Check impact with risk levels
```

#### Pattern 3: Debugging Error
```
1. get_context (line=error_line)                → Find function
2. read_focused_code (focus_symbol=func_name)   → See implementation
3. find_usages (symbol=variable_name)           → Trace data flow
```

#### Pattern 4: Understanding Large File
```
1. file_shape ()                                → See all functions
2. read_focused_code (focus_symbol=main_func)   → Start with entry point
3. read_focused_code (focus_symbol=helper)      → Drill into helpers as needed
```

### Token Optimization Strategies

- **Low Budget (<2000 tokens):** Use `file_shape` instead of `parse_file`, `code_map` with `detail="minimal"`, set `find_usages` `max_context_lines=20`
- **Medium Budget (2000-5000 tokens):** Use `read_focused_code` for focused editing, default settings
- **High Budget (>5000 tokens):** Use `parse_file` freely, `code_map` with `detail="full"`

### Common Anti-Patterns (What NOT to Do)

❌ **Using parse_file for quick overview** → Use `file_shape` instead (10x cheaper)  
❌ **Using query_pattern for symbol search** → Use `find_usages` instead (simpler, cross-language)  
❌ **Using parse_file on large files (>500 lines) without checking file_shape first** → Always start with `file_shape`  
❌ **Not setting max_context_lines when using find_usages on common symbols** → Can cause token explosion  
❌ **Using get_node_at_position when you just need function name** → Use `get_context` instead (simpler)

---

### 1. parse_file

Parse single file with FULL implementation details. Returns complete code for all functions/classes.

**Use When:**
- ✅ Understanding implementation details before editing
- ✅ File is <500 lines and you need complete context
- ✅ Writing tests that require understanding function logic
- ✅ Modifying multiple functions in same file

**Don't Use When:**
- ❌ You only need function names/signatures → use `file_shape` (10x cheaper)
- ❌ You only need to edit one function → use `read_focused_code` (3x cheaper)
- ❌ File is >500 lines and you need overview → use `file_shape` first
- ❌ Exploring multiple files → use `code_map`

**Token Cost:** HIGH (full file contents)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `include_code` (boolean, optional, default: true): Set false for 60-80% token reduction (signatures only)

**Example**:
```json
{
  "file_path": "/path/to/file.rs",
  "include_code": true
}
```

**Returns**: Complete structure with function/class names, signatures, line ranges, doc comments, and code blocks

**Optimization:** Set `include_code=false` to reduce by 60-80% (equivalent to `file_shape`)

---

### 2. file_shape

Extract file structure WITHOUT implementation code. Returns skeleton: function/class signatures, imports, dependencies only (NO function bodies).

**Use When:**
- ✅ Quick overview of file's API/interface
- ✅ Deciding which function to focus on before using `read_focused_code`
- ✅ Mapping dependencies between files (use `include_deps=true`)
- ✅ File is >500 lines and you need to orient yourself

**Don't Use When:**
- ❌ You need implementation logic → use `parse_file` or `read_focused_code`
- ❌ Exploring multiple files → use `code_map`
- ❌ You already know which function to edit → use `read_focused_code` directly

**Token Cost:** LOW (10-20% of parse_file)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `include_deps` (boolean, optional, default: false): Include project dependencies as nested file shapes
- `merge_templates` (boolean, optional, default: false): For templates in `templates/` dir: merge extends/includes into single output

**Example**:
```json
{
  "file_path": "/path/to/lib.rs",
  "include_deps": true
}
```

**Optimization:** Use this FIRST, then drill down with `parse_file` or `read_focused_code`

**Typical Workflow:** `file_shape` → `read_focused_code` (specific function) → `parse_file` (if needed)

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

Generate hierarchical map of a DIRECTORY (not single file). Returns structure overview of multiple files.

**Use When:**
- ✅ First time exploring unfamiliar codebase
- ✅ Finding where functionality lives across multiple files
- ✅ Getting project structure overview
- ✅ You don't know which file to examine

**Don't Use When:**
- ❌ You know the specific file → use `file_shape` or `parse_file`
- ❌ You need implementation details → use `parse_file` after identifying files
- ❌ Analyzing a single file → use `file_shape`

**Token Cost:** MEDIUM (scales with project size)

**Parameters**:
- `path` (string, required): Path to file or directory
- `max_tokens` (integer, optional, default: 2000): Maximum tokens for output (budget limit to prevent overflow)
- `detail` (string, optional, default: "signatures"): Detail level - "minimal" (names only), "signatures" (names + signatures), "full" (includes code)
- `pattern` (string, optional): Glob pattern to filter files (e.g., "*.rs", "src/**/*.ts")

**Example**:
```json
{
  "path": "/path/to/project/src",
  "max_tokens": 3000,
  "detail": "signatures",
  "pattern": "*.rs"
}
```

**Optimization:** Start with `detail="minimal"` for large projects, use `pattern` to filter

**Typical Workflow:** `code_map` → `file_shape` (specific files) → `parse_file`/`read_focused_code`

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

### 4. read_focused_code

Read file with FULL code for ONE symbol, signatures-only for everything else. Optimized for focused editing.

**Use When:**
- ✅ You know exactly which function to edit
- ✅ You need surrounding context to understand dependencies
- ✅ File is large but you only care about one function
- ✅ You want to minimize tokens while maintaining context

**Don't Use When:**
- ❌ You need to understand multiple functions → use `parse_file`
- ❌ You don't know which function to focus on → use `file_shape` first
- ❌ You need all implementations → use `parse_file`

**Token Cost:** MEDIUM (one function + file skeleton, ~30% of parse_file)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `focus_symbol` (string, required): Function/class/struct name to show full code for
- `context_radius` (integer, optional, default: 0): Include full code for N symbols before/after the focused symbol

**Example**:
```json
{
  "file_path": "/path/to/calculator.rs",
  "focus_symbol": "add",
  "context_radius": 0
}
```

**Returns**: Complete implementation of target function/class plus signatures of surrounding code

**Optimization:** Keep `context_radius=0` unless you need adjacent functions

**Typical Workflow:** `file_shape` (find function name) → `read_focused_code` (edit it)

---

### 5. find_usages

Find ALL usages of a symbol (function, variable, class, type) across files. Semantic search, not text search.

**Use When:**
- ✅ Refactoring: need to see all places that call a function
- ✅ Impact analysis: checking what breaks if you change a signature
- ✅ Tracing data flow: where does this variable get used?
- ✅ Before renaming or modifying shared code

**Don't Use When:**
- ❌ You need structural changes only → use `parse_diff`
- ❌ You want risk assessment → use `affected_by_diff` (includes risk levels)
- ❌ You need complex pattern matching → use `query_pattern`
- ❌ Symbol is used in >50 places → use `affected_by_diff` or set `max_context_lines=50`

**Token Cost:** MEDIUM-HIGH (scales with usage count × context_lines)

**Parameters**:
- `symbol` (string, required): Symbol name to search for
- `path` (string, required): File or directory path to search in
- `context_lines` (integer, optional, default: 3): Lines of context around each usage
- `max_context_lines` (integer, optional): Cap total context to prevent token explosion

**Example**:
```json
{
  "symbol": "helper_fn",
  "path": "/path/to/project",
  "context_lines": 3,
  "max_context_lines": 50
}
```

**Optimization:** Set `max_context_lines=50` for frequently-used symbols, or `context_lines=1` for locations only

**Typical Workflow:** `find_usages` (before changes) → make changes → `affected_by_diff` (verify impact)

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

### 6. parse_diff

Analyze structural changes vs git revision. Returns symbol-level diff (functions/classes added/removed/modified), not line-level.

**Use When:**
- ✅ Verifying what you changed at a structural level
- ✅ Checking if changes are cosmetic (formatting) or substantive
- ✅ Understanding changes without re-reading entire file
- ✅ Generating change summaries

**Don't Use When:**
- ❌ You need to see what might break → use `affected_by_diff`
- ❌ You haven't made changes yet → use `parse_file`
- ❌ You need line-by-line diff → use `git diff`

**Token Cost:** LOW-MEDIUM (much smaller than re-reading file)

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

**Typical Workflow:** After changes: `parse_diff` (verify) → `affected_by_diff` (check impact)

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

### 7. affected_by_diff

Find usages AFFECTED by your changes. Combines `parse_diff` + `find_usages` to show blast radius with risk levels.

**Use When:**
- ✅ After modifying function signatures - what might break?
- ✅ Before running tests - anticipate failures
- ✅ During refactoring - understand impact radius
- ✅ Risk assessment for code changes

**Don't Use When:**
- ❌ You haven't made changes yet → use `find_usages` first
- ❌ You just want to see what changed → use `parse_diff`
- ❌ Changes are purely internal (no signature changes) → `parse_diff` is enough

**Token Cost:** MEDIUM-HIGH (combines parse_diff + find_usages)

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

**Optimization:** Use `scope` parameter to limit search area

**Typical Workflow:** `parse_diff` (see changes) → `affected_by_diff` (assess impact) → fix issues

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

### 8. query_pattern

Execute custom tree-sitter S-expression query for advanced AST pattern matching. Returns matches with code context for complex structural patterns.

**Use When:**
- ✅ Finding all instances of specific syntax pattern (e.g., all if statements)
- ✅ Complex structural queries (e.g., all async functions with try-catch)
- ✅ Language-specific patterns `find_usages` can't handle
- ✅ You know tree-sitter query syntax

**Don't Use When:**
- ❌ Finding function/variable usages → use `find_usages` (simpler, cross-language)
- ❌ You don't know tree-sitter syntax → use `find_usages` or `parse_file`
- ❌ Simple symbol search → use `find_usages`

**Token Cost:** MEDIUM (depends on match count)

**Complexity:** HIGH - requires tree-sitter query knowledge

**Recommendation:** Prefer `find_usages` for 90% of use cases

**Parameters**:
- `file_path` (string, required): Path to the source file
- `query` (string, required): Tree-sitter query in S-expression format
- `context_lines` (integer, optional, default: 2): Lines around each match

**Example**:
```json
{
  "file_path": "/path/to/file.rs",
  "query": "(function_item name: (identifier) @name)",
  "context_lines": 2
}
```

**Optimization:** Make queries as specific as possible to reduce matches

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

---

### 9. get_context

Get enclosing scope hierarchy at specific file:line:column position. Returns nested contexts from innermost to outermost.

**Use When:**
- ✅ You have a line number from an error, stack trace, or user reference
- ✅ You need to know "what function is this line in?"
- ✅ Understanding scope hierarchy for debugging
- ✅ Navigating to a specific location in code

**Don't Use When:**
- ❌ You need the actual code → use `read_focused_code` after getting function name
- ❌ You need detailed AST info → use `get_node_at_position`
- ❌ You know the function name already → use `read_focused_code` directly

**Token Cost:** LOW (just scope chain)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `line` (integer, required): Line number (1-indexed)
- `column` (integer, optional, default: 1): Column number (1-indexed)

**Example**:
```json
{
  "file_path": "/path/to/file.rs",
  "line": 42,
  "column": 15
}
```

**Returns**: Nested contexts from innermost to outermost (e.g., "inside function X, inside class Y, inside module Z")

**Typical Workflow:** `get_context` (find function) → `read_focused_code` (see implementation)

---

### 10. get_node_at_position

Get precise AST node at file:line:column position with parent chain. Returns node type, text, range, and ancestor nodes.

**Use When:**
- ✅ You need exact syntactic information at a cursor position
- ✅ Syntax-aware edits (e.g., "wrap this expression in a function call")
- ✅ Understanding what token/expression is at a location
- ✅ Debugging parse issues or AST structure

**Don't Use When:**
- ❌ You just need to know the function name → use `get_context` (simpler)
- ❌ You need the full function code → use `read_focused_code`
- ❌ You're not doing syntax-aware operations → use `get_context`

**Token Cost:** LOW (just node info)

**Complexity:** MEDIUM - requires understanding AST concepts

**Use Case:** Advanced/syntax-aware operations only

**Parameters**:
- `file_path` (string, required): Path to the source file
- `line` (integer, required): Line number (1-indexed)
- `column` (integer, required): Column number (1-indexed)
- `ancestor_levels` (integer, optional, default: 3): Number of ancestor levels to return

**Example**:
```json
{
  "file_path": "/path/to/file.rs",
  "line": 42,
  "column": 15,
  "ancestor_levels": 3
}
```

**Returns**: Node type, text, range, and N ancestor nodes

**Optimization:** Reduce `ancestor_levels` if you don't need deep hierarchy

---

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
