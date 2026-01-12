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
- **Swift** (.swift)
- **C#** (.cs)
- **Java** (.java)
- **Go** (.go)

## Installation

### Prerequisites

- Rust toolchain (1.70 or later)
- Cargo (comes with Rust)

### Build from Source

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd treesitter-mcp
   ```

2. Build the release binary:
   ```bash
   cargo build --release
   ```

   The compiled binary will be located at `target/release/treesitter-mcp`.

## Configuration

### Claude Desktop

To configure the server for Claude Desktop, edit your configuration file:

- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

Add the `treesitter-mcp` entry to `mcpServers`:

```json
{
  "mcpServers": {
    "treesitter-mcp": {
      "command": "/ABSOLUTE/PATH/TO/treesitter-mcp",
      "args": []
    }
  }
}
```

*Note: Replace `/ABSOLUTE/PATH/TO/` with the full absolute path to your cloned repository.*

### Other MCP Clients

For any other MCP client, configure it to run the binary directly:

```bash
/path/to/treesitter-mcp
```

Alternatively, you can run it via Cargo (slower startup):

```bash
cargo run --release --manifest-path /path/to/treesitter-mcp/Cargo.toml
```

## Running Manually

The server communicates via `stdio` (standard input/output). You can run it manually to verify it starts (it will wait for JSON-RPC messages):
## Available Tools

### Quick Tool Selection Guide

Choose the right tool for your task:

#### "I need to understand code"
- **Don't know which file?** → `code_map` (directory overview)
- **Starting a new session?** → `type_map` (usage-ranked type context)
- **Know the file, need overview?** → `view_code` with `detail="signatures"` (signatures only)
- **Know the file, need full details?** → `view_code` with `detail="full"` (complete code)
- **Know the specific function?** → `view_code` with `focus_symbol` (focused view, optimized tokens)

#### "I need to find something"
- **Where is symbol X used?** → `find_usages` (semantic search with usage types)
- **Complex pattern matching?** → `query_pattern` (advanced, requires tree-sitter syntax)
- **What function is at line N?** → `symbol_at_line` (symbol info with scope hierarchy)
- **What data is available in a template?** → `template_context` (Askama template variables)

#### "I'm refactoring/changing code"
- **Before changes:** `find_usages` (see all usages)
- **After changes:** `parse_diff` (verify changes at symbol level)
- **Impact analysis:** `affected_by_diff` (what might break with risk levels)

### Tool Comparison Matrix

| Tool | Scope | Token Cost | Speed | Best For |
|------|-------|------------|-------|----------|
| `type_map` | Directory | Medium | Fast | LLM context priming, finding key types |
| `code_map` | Directory | Medium | Fast | First-time exploration |
| `view_code` (signatures) | Single file | Low | Fast | Quick overview, API understanding |
| `view_code` (full) | Single file | High | Fast | Deep understanding, multiple functions |
| `view_code` (focused) | Single file | Medium | Fast | Editing specific function |
| `find_usages` | Multi-file | Medium-High | Medium | Refactoring, impact analysis |
| `affected_by_diff` | Multi-file | Medium-High | Medium | Post-change validation |
| `parse_diff` | Single file | Low-Medium | Fast | Verify changes |
| `symbol_at_line` | Single file | Low | Fast | Error debugging, scope lookup |
| `query_pattern` | Single file | Medium | Medium | Complex patterns (advanced) |
| `template_context` | Single file | Low-Medium | Fast | Askama template editing |

### Common Workflow Patterns

#### Pattern 1: LLM Session Initialization
```
1. type_map (path="src", max_tokens=3000)      → Get usage-ranked types
2. code_map (path="src", detail="minimal")      → Get file structure
3. Begin coding tasks with full type awareness
```

#### Pattern 2: Exploring New Codebase
```
1. code_map (path="src", detail="minimal")      → Get lay of the land
2. view_code (detail="signatures")              → Understand interfaces
3. view_code (focus_symbol="function_name")     → Deep dive
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
1. symbol_at_line (line=error_line)             → Find function
2. view_code (focus_symbol=func_name)           → See implementation
3. find_usages (symbol=variable_name)           → Trace data flow
```

#### Pattern 4: Understanding Large File
```
1. view_code (detail="signatures")              → See all functions
2. view_code (focus_symbol=main_func)           → Start with entry point
3. view_code (focus_symbol=helper)              → Drill into helpers as needed
```

### Token Optimization Strategies

- **Low Budget (<2000 tokens):** Use `view_code` with `detail="signatures"`, `code_map` with `detail="minimal"`, set `find_usages` `max_context_lines=20`
- **Medium Budget (2000-5000 tokens):** Use `view_code` with `focus_symbol` for focused editing, default settings
- **High Budget (>5000 tokens):** Use `view_code` with `detail="full"` freely, `code_map` with `detail="full"`

### Common Anti-Patterns (What NOT to Do)

❌ **Using view_code with detail="full" for quick overview** → Use `detail="signatures"` instead (10x cheaper)  
❌ **Using query_pattern for symbol search** → Use `find_usages` instead (simpler, cross-language)  
❌ **Using view_code with detail="full" on large files without checking signatures first** → Always start with `detail="signatures"`  
❌ **Not setting max_context_lines when using find_usages on common symbols** → Can cause token explosion  
❌ **Not using focus_symbol when editing specific functions** → Use `focus_symbol` for 3x token savings

---

### 1. type_map

Generate a usage-sorted map of all project types. Returns structs, classes, enums, interfaces, traits, protocols, and type aliases prioritized by usage frequency.

**Primary Use Case:** Provide LLM agents with comprehensive type context at session start to prevent hallucinations about type names, fields, and signatures.

**Use When:**
- ✅ Starting an LLM coding session (context priming)
- ✅ Need accurate type definitions across entire project
- ✅ Want to understand which types are most important

**Don't Use When:**
- ❌ Need function/method implementations → use `view_code`
- ❌ Need call hierarchy or control flow → use `code_map`
- ❌ Analyzing a single file → use `view_code`

**Token Cost:** MEDIUM (2000-3000 tokens typical for medium projects)

**Parameters:**
- `path` (string, required): Directory to scan
- `max_tokens` (integer, optional, default: 2000): Token budget (tiktoken counted)
- `pattern` (string, optional): Glob filter (e.g., `"*.rs"`, `"src/**/*.ts"`)

**Returns:** Compact schema (usage-sorted types)

- Output keys: `h` (header) and `types` (rows: `name|kind|file|line|usage_count`)
- Optional meta: `@` (e.g. `@.t=true` when truncated)
- Rows are newline-delimited; fields are pipe-delimited and escaped (`\\`, `\n`, `\r`, `\|`)

```json
{
  "h": "name|kind|file|line|usage_count",
  "types": "User|struct|src/domain/models.rs|11|42\nOrder|struct|src/domain/models.rs|107|37"
}
```

---

### 2. view_code

View a source file with flexible detail levels and automatic type inclusion from project dependencies.

**Use When:**
- ✅ Need to view/edit a file
- ✅ Want type definitions from dependencies
- ✅ Need full code or just signatures
- ✅ Editing specific function (use `focus_symbol`)

**Don't Use When:**
- ❌ Exploring multiple files → use `code_map`
- ❌ You haven't identified the file yet → use `code_map` first

**Token Cost:** MEDIUM-HIGH (varies by detail level)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `detail` (string, optional, default: "full"): Detail level
  - `"signatures"`: Function/class signatures only (no bodies) - 10x cheaper
  - `"full"`: Complete implementation code
- `focus_symbol` (string, optional): Focus on ONE symbol, show full code only for it
  - When set, returns full code for this symbol + signatures for rest - 3x cheaper

**Auto-Includes**: All struct/class/interface definitions from project dependencies (not external libs)

**Returns**: Compact schema (BREAKING).

- Output keys: `p` (relative path) plus row tables (`h`/`f`/`s`/`c`), and optional additional tables (`ih`/`im`, `bh`/`bm`, etc.)
- Optional meta: `@` (e.g. `@.t=true` when truncated)

```json
{
  "p": "src/calculator.rs",
  "h": "name|line|sig",
  "f": "add|5|pub fn add(a: i32, b: i32) -> i32",
  "s": "Calculator|15|pub struct Calculator"
}
```

**Optimization:** 
- Use `detail="signatures"` for quick overview (10x cheaper)
- Use `focus_symbol` for focused editing (3x cheaper)

**Typical Workflow:** `code_map` → `view_code`

---

### 3. code_map

Generate hierarchical map of a DIRECTORY (not single file). Returns structure overview of multiple files.

**Use When:**
- ✅ First time exploring unfamiliar codebase
- ✅ Finding where functionality lives across multiple files
- ✅ Getting project structure overview
- ✅ You don't know which file to examine

**Don't Use When:**
- ❌ You know the specific file → use `view_code`
- ❌ You need implementation details → use `view_code` after identifying files
- ❌ Analyzing a single file → use `view_code`

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

**Typical Workflow:** `code_map` → `view_code` (signatures/full/focus)

**Returns**: Compact schema keyed by relative file paths.

- Top-level keys are file paths
- Per-file keys: `h` + optional `f`/`s`/`c` row strings
- Optional meta: `@` (e.g. `@.t=true` when truncated)

```json
{
  "src/main.rs": {
    "h": "name|line|sig",
    "f": "main|10|fn main()\ninitialize|25|fn initialize()"
  },
  "src/config.rs": {
    "h": "name|line|sig",
    "s": "Config|5|pub struct Config"
  }
}
```

---

### 4. find_usages

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

**Returns**: Compact schema.

- Output keys: `sym` (symbol), `h` (header), `u` (usage rows)
- Optional meta: `@` (e.g. `@.t=true` when truncated)

```json
{
  "sym": "helper_fn",
  "h": "file|line|col|type|context",
  "u": "src/main.rs|42|15|call|let result = helper_fn();\nsrc/utils.rs|18|9|reference|helper_fn() + 10"
}
```

---

### 5. symbol_at_line

Get symbol (function/class/method) at specific line with signature and scope chain.

**Use When:**
- ✅ Have line number from error/stack trace
- ✅ Need to know "what function is this line in?"
- ✅ Want function signature at a location
- ✅ Understanding scope hierarchy

**Don't Use When:**
- ❌ Need full code → use `view_code` with `focus_symbol`
- ❌ Know symbol name already → use `view_code` directly

**Token Cost:** LOW

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

**Returns**: Compact schema.

- Output keys: `sym` (symbol name), `kind` (abbrev), `sig` (signature), `l` (line), `scope` (scope chain)

```json
{
  "sym": "calculate",
  "kind": "fn",
  "sig": "pub fn calculate(x: i32) -> i32",
  "l": 40,
  "scope": "math::Calculator::calculate"
}
```

**Typical Workflow:** `symbol_at_line` (find symbol) → `view_code` (see code)

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
- ❌ You haven't made changes yet → use `view_code`
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

**Returns**: Compact schema.

- Output keys: `p` (relative file path), `cmp` (compare_to), `h` (header), `changes` (rows)

```json
{
  "p": "src/calculator.rs",
  "cmp": "HEAD",
  "h": "type|name|line|change",
  "changes": "fn|add|15|sig_changed: fn add(a: i64, b: i64) -> i64\nfn|multiply|25|added"
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

**Returns**: Compact schema.

- Output keys: `p` (relative file path), `h` (header), `affected` (rows)
- `risk` is one of: `high` | `medium` | `low`

```json
{
  "p": "src/calculator.rs",
  "h": "symbol|change|file|line|risk",
  "affected": "add|sig_changed|src/main.rs|42|high\nadd|sig_changed|tests/calculator_test.rs|15|high"
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
- ❌ You don't know tree-sitter syntax → use `find_usages` or `view_code`
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

**Returns**: Compact schema.

- Output keys: `q` (query), `h` (header), `m` (match rows)

```json
{
  "q": "(function_item name: (identifier) @name)",
  "h": "file|line|col|text",
  "m": "src/calculator.rs|5|8|add\nsrc/calculator.rs|10|8|multiply"
}
```

---

### 9. template_context

Find Rust structs associated with an Askama template file. Returns struct names, fields, and types (resolved up to 3 levels deep) that are available as variables in the template.

**Use When:**
- ✅ Editing Askama HTML templates and need to know available variables
- ✅ Understanding what data is passed to a template
- ✅ Debugging template rendering issues

**Don't Use When:**
- ❌ Not using Askama templates
- ❌ Working with non-template files

**Token Cost:** LOW-MEDIUM

**Parameters**:
- `template_path` (string, required): Path to the template file (relative or absolute)

**Example**:
```json
{
  "template_path": "templates/calculator.html"
}
```

**Returns**: Compact schema.

- Output keys: `tpl` (relative template path)
- Context rows: `h` + `ctx` (rows: `struct|field|type`)
- Struct locations: `sh` + `s` (rows: `struct|file|line`)

```json
{
  "tpl": "templates/calculator.html",
  "h": "struct|field|type",
  "ctx": "CalculatorContext|result|i32\nCalculatorContext|history|Vec<HistoryEntry>",
  "sh": "struct|file|line",
  "s": "CalculatorContext|src/templates.rs|12"
}
```

**Typical Workflow:** `template_context` → edit template with known variables

---

## Performance Considerations

- **Parsing**: Tree-sitter parsers are highly optimized and can handle large files efficiently
- **Token Limits**: The `code_map` tool respects token budgets to avoid overwhelming AI context windows
- **Caching**: Parsed trees are not cached between requests; prefer `view_code` with `detail="signatures"` for repeated lightweight reads
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
