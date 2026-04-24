# Tree-sitter MCP Server

AST-first MCP for coding agents. Instead of pasting raw files into the context window, it returns compact structural answers: signatures, usage rows, focused edit context, impact summaries, and review bundles with explicit token budgets.

![Token efficiency comparison: MCP vs agent-style shell baselines](docs/token-efficiency-comparison.svg)

## What You Get

Numbers below compare MCP payloads against the shell workflow an agent would actually
reach for (grep, find + head, targeted reads) — not against cat-everything straw-men.

- **File overview ~2.7× smaller** than `cat file` (signatures instead of bodies).
- **Focused edit ~5.0× smaller** than `cat file` (one symbol plus direct deps, not the whole file).
- **Repo search ~4.5× smaller** than `grep -rn -C3 symbol`, with added scope, owner, and usage-type per hit.
- **Directory map ~3.2× smaller** than `find -type f + head -50` per file, and structured rather than raw text.
- **Call-graph tracing ~57× smaller** than grepping then cat-ing each matched file (the workflow for "who calls X?").
- **~2,071 tokens** added to the agent context window to register the server.
- **Payload-size regressions fail CI** — if a refactor bloats a tool output, the build breaks.

These are indicative averages over 18 runs across Rust, TypeScript, Python, and JavaScript
fixtures — treat them as "order-of-magnitude", not precise guarantees. Token savings alone
do not prove a quality win; see [BENCHMARK.md](BENCHMARK.md) for the accuracy-benchmark
methodology this project is building out.

## What It Does

`treesitter-mcp` reduces token load with four repeated patterns:

1. **Structural filtering**: return AST-derived symbols and signatures instead of raw bodies when full code is unnecessary.
2. **Focused extraction**: return one symbol plus only the dependencies, imports, types, and tests that matter for that task.
3. **Compact grouping**: use stable, row-oriented schemas so repeated keys and prose do not dominate the payload.
4. **Budget-aware truncation**: use `tiktoken` counts and explicit `max_tokens` limits to keep results bounded.

This is the core positioning: it is not just a parser, it is a **context compressor for code workflows**.

## Installation

### Homebrew (macOS)

```bash
brew tap christoph/treesitter-mcp
brew install treesitter-mcp

claude mcp add --scope project treesitter-mcp -- /opt/homebrew/bin/treesitter-mcp
```

### Release Binaries (Linux and Windows)

Prebuilt release binaries are available on the [GitHub Releases](https://github.com/Christoph/treesitter-mcp/releases/latest) page.

- Linux: download the release archive for your target, extract it, and point your MCP client at the `treesitter-mcp` binary
- Windows: download the Windows release archive, extract it, and point your MCP client at `treesitter-mcp.exe`

### Other Source Builds

If you are not using Homebrew or a release binary, the same `cargo build --release` flow also works on other supported platforms with a working Rust toolchain.

## Configuration

### Claude Code CLI

Add the server to the current project:

```bash
claude mcp add --scope project treesitter-mcp -- /ABSOLUTE/PATH/TO/treesitter-mcp
```

You can verify that Claude Code sees it with:

```bash
claude mcp list
```

Or add it directly in a project-level `.mcp.json`:

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

Once connected, ask Claude Code to use it explicitly, for example:

```text
Use treesitter-mcp to map the src directory, then inspect the service layer before proposing changes.
```

### Codex

Add the server to `~/.codex/config.toml`:

```toml
[mcp_servers.treesitter-mcp]
command = "/ABSOLUTE/PATH/TO/treesitter-mcp"
```

Then restart Codex and confirm it is available:

```bash
codex mcp list
```

Once configured, prompt Codex to use the MCP directly, for example:

```text
Use treesitter-mcp to find all usages of UserService, then show the smallest edit context for update_user.
```

### Other MCP Clients

For any other MCP client, configure it to run the binary directly:

```bash
/path/to/treesitter-mcp
```

Alternatively, you can run it via Cargo (slower startup):

```bash
cargo run --release --manifest-path /path/to/treesitter-mcp/Cargo.toml
```


Build the binary:

```bash
cargo build --release
```

Point your MCP client at `target/release/treesitter-mcp`, then start with a small workflow instead of raw reads:

```text
1. code_map(path="src", detail="minimal", with_types=true)
2. view_code(file_path="...", detail="signatures")
3. minimal_edit_context(file_path="...", symbol_name="...")
4. review_context(file_path="...") after changes
```

## Quick Start

If you need the full installation and configuration details, keep reading below. For the messaging and roadmap behind this README, see [docs/COMMUNICATION.md](docs/COMMUNICATION.md).

## Token Efficiency Comparison

Measured on the current code after rebuilding the server.
Baselines emulate the shell workflow an agent would actually run (grep, find + head,
targeted reads) — **not** `cat <every file in scope>`. The MCP side is the exact JSON
payload returned by each tool, the same shape the built MCP server returns.
All token counts below are **averages**, not single examples.

| Workflow average | Samples | Agent-style baseline | MCP tool | Raw avg tokens | MCP avg tokens | Saved avg tokens | Saved | Smaller |
|---|---|---:|---:|---:|---:|---:|
| Overview average | 4 | `cat <source file>` | `view_code(detail="signatures")` | 852 | 314 | 538 | 63.1% | 2.7x |
| Focused edit average | 4 | `cat <source file>` | `minimal_edit_context(symbol_name=...)` | 852 | 170 | 682 | 80.0% | 5.0x |
| Call graph average | 4 | `grep -rln symbol src \| xargs cat` | `call_graph(symbol_name=...)` | 59,513 | 1,044 | 58,469 | 98.2% | 57.0x |
| Repo search average | 3 | `grep -rn -C3 symbol src/analysis` | `find_usages(symbol=...)` | 3,803 | 837 | 2,966 | 78.0% | 4.5x |
| Directory map average | 3 | `find -type f + head -50 each file` | `code_map(detail="minimal")` | 8,978 | 2,783 | 6,195 | 69.0% | 3.2x |

Saved avg tokens = raw avg tokens - MCP avg tokens. Percent saved = `1 - MCP/raw`.

Notes:
- Repo search baseline is `grep -rn -C3`, not bare `grep -l`. Bare grep returns only
  locations, so it would appear cheaper than MCP for pure locate; the fair comparison
  is "locate + a few lines of context", which is what `find_usages` returns plus scope
  and usage-type metadata.
- Call-graph baseline reads every file whose text contains the symbol, because
  tracing callers without LSP requires reading those files. This is what makes it so
  much more expensive than tools that resolve callers structurally.
- Sample sizes (3–4 per row) are small — treat multipliers as indicative, not precise.

## Use This Instead of Raw Reads

- `view_code(detail="signatures")` instead of `cat` when you need structure but not bodies.
- `minimal_edit_context` instead of focused file reads when you are editing one known symbol.
- `call_graph` instead of reading multiple files to trace one function.
- `find_usages` instead of concatenating a whole directory to answer one reference question.
- `code_map` instead of dumping a tree when you only need the project shape.
- `review_context` instead of manually assembling diff, impact, tests, and changed-symbol context.

## Communication Commitments

- The README leads with measured value, not internal architecture.
- Benchmarks are reproducible through `cargo test report_average_token_benchmarks -- --ignored --nocapture`.
- CI publishes a benchmark summary so pull requests show the token story directly in the pipeline.
- New token-saving ideas are tracked in [docs/COMMUNICATION.md](docs/COMMUNICATION.md), including opportunities still missing from the product.

## Measurement Method

The averaged benchmark uses 18 total runs:

- 4 file-overview runs across Rust, TypeScript, Python, and JavaScript fixture files
- 4 focused-edit runs across the same four source files
- 4 call-graph runs across analysis modules in this repository
- 3 repo-search runs in `src/analysis`
- 3 directory-map runs across `src`, `src/analysis`, and `tests/fixtures/complex_rust_service/src`

For each run:

- the **baseline** token count emulates the shell workflow an agent would actually run:
  - `cat file` for file-overview and focused-edit scenarios
  - `grep -rn -C3 <symbol> <scope>` for repo search
  - `grep -rln <symbol> <scope> | xargs cat` for call-graph tracing (read every file that
    mentions the symbol, since tracing callers without LSP requires reading them)
  - `find <path> -type f` listing plus `head -n 50 <file>` per source file for directory maps
- the **MCP token count** is the tool response JSON text
- both sides are counted with `tiktoken_rs::cl100k_base()`
- baselines use word-boundary matching to approximate real grep behaviour, and skip
  files whose language the server does not recognise (same filter the MCP side uses)

## Overview

Tree-sitter MCP Server exposes powerful code analysis tools through the MCP protocol, allowing AI assistants to:

- Parse and analyze code structure across multiple languages
- Extract high-level file shapes without implementation details
- Generate token-aware code maps of entire projects
- Find symbol usages across codebases
- Execute custom tree-sitter queries for advanced analysis
- Analyze structural changes between file versions (diff-aware analysis)
- Identify potentially affected code when making changes
- Adds ~2,071 tokens to the context window when adding the mcp

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

## Available Tools

### Quick Tool Selection Guide

Choose the right tool for your task:

#### "I need to understand code"
- **Don't know which file?** → `code_map` (directory overview)
- **Starting a new session?** → `type_map` (usage-ranked type context)
- **Know the file, need overview?** → `view_code` with `detail="signatures"` (signatures only)
- **Know the file, need full details?** → `view_code` with `detail="full"` (complete code)
- **Know the specific function?** → `view_code` with `focus_symbol` (focused view, optimized tokens)
- **Editing one known symbol?** → `minimal_edit_context` (smallest useful edit context)

#### "I need to find something"
- **Where is symbol X used?** → `find_usages` (syntax-aware search with usage types)
- **What calls this / what does this call?** → `call_graph` (compact best-effort callers/callees)
- **Already have LSP references?** → `format_references` (compact context for precise locations)
- **Already have LSP diagnostics?** → `format_diagnostics` (compact diagnostics with owners)
- **Complex pattern matching?** → `query_pattern` (advanced, requires tree-sitter syntax)
- **What function is at line N?** → `symbol_at_line` (symbol info with scope hierarchy)
- **What data is available in a template?** → `template_context` (Askama template variables)

#### "I'm refactoring/changing code"
- **Before editing a signature:** `preview_impact` (estimate blast radius first)
- **Before changes:** `find_usages` (see all usages)
- **After changes:** `parse_diff` (verify changes at symbol level)
- **Impact analysis:** `affected_by_diff` (what might break with risk levels)
- **Which tests should I run?** `relevant_tests` (rank likely tests for one symbol)
- **Did I only change what I meant to change?** `verify_edit` (compact structural guardrail)
- **Need reviewer context for a diff?** `review_context` (diff + impact + tests + focused context)

### Tool Comparison Matrix

| Tool | Scope | Token Cost | Speed | Best For |
|------|-------|------------|-------|----------|
| `type_map` | Directory | Medium | Fast | LLM context priming, finding key types |
| `type_map` (count_usages=false) | Directory | Medium | Faster | Type locations without usage ranking |
| `code_map` | Directory | Medium | Fast | First-time exploration |
| `code_map` (with_types=true) | Directory | Medium | Fast | Code structure + types in one pass |
| `view_code` (signatures) | Single file | Low | Fast | Quick overview, API understanding |
| `view_code` (full) | Single file | High | Fast | Deep understanding, multiple functions |
| `view_code` (focused) | Single file | Medium | Fast | Editing specific function |
| `minimal_edit_context` | Single symbol | Low | Fast | Focused edits with direct deps |
| `call_graph` | Single symbol | Low-Medium | Medium | Best-effort callers/callees |
| `preview_impact` | Single symbol + scope | Medium | Medium | Planned signature changes before editing |
| `find_usages` | Multi-file | Medium-High | Medium | Refactoring, impact analysis |
| `format_references` | LSP locations | Low-Medium | Fast | Compact context for precise LSP references |
| `format_diagnostics` | LSP diagnostics | Low-Medium | Fast | Compact diagnostics with owners |
| `affected_by_diff` | Multi-file | Medium-High | Medium | Post-change validation |
| `parse_diff` | Single file | Low-Medium | Fast | Verify changes |
| `relevant_tests` | Single symbol | Low-Medium | Fast | Targeted test selection after edits |
| `verify_edit` | Single file diff | Low | Fast | Check edit stayed within intended scope |
| `review_context` | Single file diff | Medium | Medium | Compact review bundle for changed files |
| `symbol_at_line` | Single file | Low | Fast | Error debugging, scope lookup |
| `query_pattern` | Single file | Medium | Medium | Complex patterns (advanced) |
| `template_context` | Single file | Low-Medium | Fast | Askama template editing |

### Precision vs. Heuristic

These tools provide strong guarantees based on AST structure:
- `view_code`: exact code extraction from parsed AST
- `parse_diff`: structural diff between file revisions
- `query_pattern`: precise tree-sitter AST queries
- `symbol_at_line`: scope chain from AST traversal
- `template_context`: Askama struct resolution

These tools use syntax-aware matching (best-effort, not compiler-grade):
- `find_usages`: identifier matching via tree-sitter, may match homonyms in different scopes
- `format_references`: trusts LSP-provided locations for precision, then adds syntax-aware context
- `format_diagnostics`: trusts LSP-provided diagnostics, then adds syntax-aware owner context
- `minimal_edit_context`: same-file relevance plus direct project-local dependency signatures from imports
- `call_graph`: project-local call extraction, same-file definitions preferred, not compiler-grade resolution
- `preview_impact`: virtual signature diff plus syntax-aware impact scan, no file edits required
- `affected_by_diff`: relies on `find_usages` for impact analysis
- `relevant_tests`: test discovery via file heuristics plus syntax-aware symbol matches
- `verify_edit`: structural diff guardrail, not semantic intent verification
- `review_context`: composition of existing tools; precision depends on the underlying diff/usages context
- `affected_by_diff`: relies on `find_usages` for impact analysis
- `code_map`: structural overview, scope-aware but not semantically resolved
- `type_map`: type identification via AST, usage counts are approximate

For compiler-grade symbol resolution (go-to-definition, precise find-references), use an LSP server alongside this MCP server.

### Common Workflow Patterns

#### Pattern 1: LLM Session Initialization (Optimized - Single Pass)
```
1. code_map (path="src", with_types=true, count_usages=true)  → Get both structure AND usage-ranked types
2. Begin coding tasks with full context
```

#### Pattern 1b: LLM Session Initialization (Traditional - Two Passes)
```
1. type_map (path="src", max_tokens=3000)      → Get usage-ranked types
2. code_map (path="src", detail="minimal")      → Get file structure
3. Begin coding tasks with full type awareness
```

#### Pattern 2: Exploring New Codebase
```
1. code_map (path="src", detail="minimal", with_types=true)  → Get structure + types in one pass
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

#### Pattern 2b: Planned Signature Change
```
1. preview_impact (symbol_name="function_name", new_signature="...") → Estimate fallout first
2. Make changes
3. relevant_tests (symbol_name="function_name")                      → Run focused tests
4. verify_edit (target_symbol="function_name")                       → Confirm edit stayed scoped
```

#### Pattern 2c: Reviewing a Local Diff
```
1. review_context (file_path="src/lib.rs")      → Diff + impact + tests + focused changed-symbol context
2. view_code / minimal_edit_context as needed   → Drill deeper only where needed
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
- ❌ Need both code structure AND types → use `code_map` with `with_types=true`

**Token Cost:** MEDIUM (2000-3000 tokens typical for medium projects)

**Parameters:**
- `path` (string, required): Directory to scan
- `max_tokens` (integer, optional, default: 2000): Token budget (tiktoken counted)
- `pattern` (string, optional): Glob filter (e.g., `"*.rs"`, `"src/**/*.ts"`)
- `count_usages` (boolean, optional, default: true): Count usages across the project. Set to `false` for faster results when you only need type locations without usage ranking.

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
- `definition_location` (object, optional): LSP `textDocument/definition` result or compact
  `{file,line,col}` location used to include the exact dependency type from that definition
- `comment_mode` (string, optional, default: `"none"`): Comment handling for returned code fields
  - `"none"`: Current compact behavior
  - `"leading"`: Prepend the contiguous leading comment block above returned symbol code

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
- Use `comment_mode="leading"` when rationale in comments matters more than raw token minimization

**Typical Workflow:** `code_map` → `view_code`

---

### 3. code_map

Generate hierarchical map of a DIRECTORY (not single file). Returns structure overview of multiple files with functions/classes/types.

**Use When:**
- ✅ First time exploring unfamiliar codebase
- ✅ Finding where functionality lives across multiple files
- ✅ Getting project structure overview
- ✅ You don't know which file to examine
- ✅ Need both code structure AND type definitions (use `with_types=true`)

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
- `with_types` (boolean, optional, default: false): Also extract type definitions (structs, enums, interfaces, etc.) in the same pass. More efficient than calling `type_map` separately.
- `count_usages` (boolean, optional, default: false): When `with_types=true`, also count usages for each type. Set to `true` for usage-ranked types.

**Example**:
```json
{
  "path": "/path/to/project/src",
  "max_tokens": 3000,
  "detail": "signatures",
  "pattern": "*.rs"
}
```

**Combined Mode Example** (replaces separate `code_map` + `type_map` calls):
```json
{
  "path": "/path/to/project/src",
  "max_tokens": 4000,
  "detail": "minimal",
  "with_types": true,
  "count_usages": true
}
```

**Optimization:**
- Start with `detail="minimal"` for large projects
- Use `pattern` to filter files
- Use `with_types=true` instead of calling `type_map` separately (single file walk vs two)

**Typical Workflow:** `code_map` → `view_code` (signatures/full/focus)

**Returns**: Compact schema keyed by relative file paths.

- Top-level keys are file paths
- Per-file keys: `h` + optional `f`/`s`/`c` row strings
- When `with_types=true`: includes `types` key with type definitions
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

**With `with_types=true`**:
```json
{
  "src/main.rs": { "h": "name|line|sig", "f": "main|10|fn main()" },
  "types": {
    "h": "name|kind|file|line|usage_count",
    "rows": "Config|struct|src/config.rs|5|12\nUser|struct|src/models.rs|10|8"
  }
}
```

---

### 4. find_usages

Find ALL usages of a symbol (function, variable, class, type) across files. Syntax-aware search, not text search.

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
  "h": "file|line|col|type|context|scope|conf|owner",
  "u": "src/main.rs|42|15|call|let result = helper_fn();|main|high|\nsrc/utils.rs|18|9|reference|helper_fn() + 10|Utils::apply|medium|"
}
```

---

### 5. format_references

Format precise LSP reference locations into the same compact schema as `find_usages`.

**Use When:**
- ✅ You already called LSP `textDocument/references`
- ✅ You want compact context, scope, usage type, and owner hints around precise references
- ✅ You need `find_usages`-compatible rows with `conf=high`

**Don't Use When:**
- ❌ You need MCP to discover references itself → use `find_usages`
- ❌ You need compiler diagnostics grouped by severity → use `format_diagnostics`

**Token Cost:** LOW-MEDIUM (scales with number of provided locations × context_lines)

**Parameters**:
- `symbol` (string, required): Symbol name these locations resolve to
- `references` (array, required): Either 1-based `{file,line,col}` / `{file_path,line,column}` rows or LSP `{uri,range:{start:{line,character}}}` rows
- `context_lines` (integer, optional, default: 3): Lines of context around each reference
- `max_tokens` (integer, optional): Hard output budget

**Example**:
```json
{
  "symbol": "helper_fn",
  "references": [
    {
      "uri": "file:///path/to/src/main.rs",
      "range": {
        "start": { "line": 41, "character": 14 }
      }
    }
  ],
  "context_lines": 1
}
```

**Returns**: Compact schema identical to `find_usages`.

- Output keys: `sym` (symbol), `h` (header), `u` (usage rows)
- `conf` is `high` because locations are assumed to come from precise LSP resolution

---

### 6. format_diagnostics

Format LSP diagnostics into compact rows with structural owner context.

**Use When:**
- ✅ You already have LSP `textDocument/diagnostics`
- ✅ You need a token-efficient diagnostics summary
- ✅ You want to know which function/class owns each diagnostic

**Don't Use When:**
- ❌ You need to run diagnostics itself → use LSP, compiler, or test tools
- ❌ You need non-diagnostic references → use `find_usages` or `format_references`

**Token Cost:** LOW-MEDIUM (scales with number of diagnostics)

**Parameters**:
- `diagnostics` (array, required): Either 1-based `{file,line,col}` / `{file_path,line,column}` rows or LSP `{uri,range:{start:{line,character}}}` rows with `severity`, `message`, optional `source`, and optional `code`
- `max_tokens` (integer, optional, default: 2000): Hard output budget

**Example**:
```json
{
  "diagnostics": [
    {
      "uri": "file:///path/to/src/main.rs",
      "range": {
        "start": { "line": 41, "character": 14 }
      },
      "severity": 1,
      "message": "cannot find value `foo` in this scope",
      "source": "rustc",
      "code": "E0425"
    }
  ],
  "max_tokens": 2000
}
```

**Returns**: Compact schema.

- Output keys: `h` (header), `d` (diagnostic rows)
- Diagnostic rows: `severity|file|line|col|owner|source|code|message`

```json
{
  "h": "severity|file|line|col|owner|source|code|message",
  "d": "error|src/main.rs|42|15|run|rustc|E0425|cannot find value `foo` in this scope"
}
```

---

### 7. minimal_edit_context

Return the smallest useful context for editing one known symbol.

**Use When:**
- ✅ Editing one known function or method
- ✅ You need the target code plus directly relevant callees, types, and imports
- ✅ `view_code(focus_symbol=...)` is still too large for a file with many symbols

**Don't Use When:**
- ❌ Exploring an unfamiliar file → use `code_map` or `view_code`
- ❌ You need full transitive project-wide dependency resolution → use `view_code(include_deps=true)` or LSP

**Token Cost:** LOW (usually much smaller than focused `view_code` on large files)

**Parameters**:
- `file_path` (string, required): Path to the source file
- `symbol_name` (string, required): Symbol to edit
- `max_tokens` (integer, optional, default: 2000): Hard output budget
- `comment_mode` (string, optional, default: `"none"`): Comment handling for the target code row
  - `"none"`: Current compact behavior
  - `"leading"`: Prepend the contiguous leading comment block above the target symbol

**Example**:
```json
{
  "file_path": "/path/to/src/workflow.ts",
  "symbol_name": "buildSummary",
  "comment_mode": "leading",
  "max_tokens": 2000
}
```

**Returns**: Compact schema.

- `target`: full code row for the symbol (`name|line|sig|code`)
- `deps`: optional same-file and direct project-local dependency signature rows (`kind|name|line|sig`)
- `types`: optional same-file referenced type rows (`kind|name|line|sig`)
- `imports`: optional relevant import rows (`line|text`)
- `scope`: enclosing class/impl scope when available

Use `comment_mode="leading"` when the target symbol has important rationale in comments above the declaration.

---

### 8. call_graph

Return compact best-effort callers and callees for one function or method.

**Use When:**
- ✅ You need to know what calls a symbol
- ✅ You need to know what the symbol calls
- ✅ You want compact depth-1 navigation or impact context without manual multi-file reads

**Don't Use When:**
- ❌ You need compiler-grade resolution across imports, generics, traits, or overloads → use LSP when available
- ❌ You are looking for non-call references → use `find_usages`

**Token Cost:** LOW-MEDIUM

**Parameters**:
- `file_path` (string, required): Path to the source file containing the symbol
- `symbol_name` (string, required): Function or method to analyze
- `direction` (string, optional, default: `"both"`): `"callers"`, `"callees"`, or `"both"`
- `depth` (integer, optional, default: 1, max: 3): Traversal depth
- `max_tokens` (integer, optional, default: 2000): Hard output budget

**Example**:
```json
{
  "file_path": "/path/to/src/workflow.rs",
  "symbol_name": "build_report",
  "direction": "both",
  "depth": 1,
  "max_tokens": 2000
}
```

**Returns**: Compact schema.

- Output keys: `sym` (symbol), `h` (header), `edges` (edge rows)
- Edge rows: `direction|symbol|file|line|scope|depth`

```json
{
  "sym": "build_report",
  "h": "direction|symbol|file|line|scope|depth",
  "edges": "callee|normalize_input|src/workflow.rs|8||1\ncaller|render_page|src/workflow.rs|20||1"
}
```

---

### 9. symbol_at_line

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

### 10. parse_diff

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

### 11. affected_by_diff

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

### 12. query_pattern

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

### 13. template_context

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

### Single-Pass Optimizations

Both `code_map` and `type_map` have been optimized for single-pass file traversal:

| Operation | Before | After |
|-----------|--------|-------|
| `type_map` with usage counting | 2 file walks | 1 file walk |
| `type_map` without usage counting | 2 file walks | 1 file walk (faster) |
| `code_map` + `type_map` separately | 3 file walks | N/A |
| `code_map` with `with_types=true` | N/A | 1 file walk |

**Recommendations:**
- Use `type_map` with `count_usages=false` when you only need type locations (skip usage counting)
- Use `code_map` with `with_types=true` instead of calling both tools separately
- The combined mode reads each file only once for both code structure and type extraction

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
