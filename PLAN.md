# Simplified Token Reduction Implementation Plan

## Overview

**Goal:** Reduce token usage by ~40–50% by optimizing JSON schemas and replacing repetitive object arrays with compact, pipe-delimited rows.

**Approach:** Single breaking schema change (no dual-format maintenance). All tools converge on:
- **Abbreviated keys**
- **Grouped-by-file** where relevant
- **Header + rows encoding** (`h` describes columns, rows are newline-delimited)
- **Early budget enforcement** (avoid “build then truncate”)

---

## Implementation Status (as of 2026-01-11)

### Shared Infrastructure (DONE)

Implemented in `src/common/*`:
- `src/common/format.rs`: pipe + newline safe row encoding
  - Escapes `\\` first, then `\n`, `\r`, then `|` (to `\\|`)
- `src/common/budget.rs`: lightweight budget tracker + conservative token estimate
- `src/common/compact.rs`: row builder helper (used where convenient)

### Tool Migrations (DONE)

These tools already emit the new compact schema and have updated tests:
- `code_map`
- `find_usages`
- `query_pattern`
- `symbol_at_line`
- `parse_diff`
- `affected_by_diff`

All checks currently pass:
- `cargo test`
- `cargo clippy -- -D warnings`

### Remaining Work (PENDING)

These tools are still on the older verbose schema:
- `view_code`
- `template_context`
- `type_map`

---

## Compact Encoding Conventions (Current)

### 1) Meta / Truncation
Tools may add a meta object under `"@"`.
- `{"@": {"t": true}}` indicates truncated output.

### 2) Row Encoding
- Rows are separated by literal newline `\n`.
- Fields are separated by literal pipe `|`.
- Field escaping is performed so the row format stays parseable:
  - `\\` → `\\\\`
  - `\n` → `\\n`
  - `\r` → `\\r`
  - `|` → `\\|`

### 3) Parsing Instructions (for clients/LLMs)
1. Read header from `h` and split by `|` to get column order.
2. Read data from `d` / `f` / `u` / `m` (tool-specific) and split by newline into rows.
3. Split each row by `|` while honoring backslash escapes.

---

## Tool Schemas (Implemented)

### 1) `code_map` (DONE)

**Output (compact):**
- Top-level JSON object keyed by *relative* file path.
- Each file entry:
  - `h`: header string
  - `f`: functions rows
  - `s`: structs rows
  - `c`: classes rows
- Optional `@` meta for truncation.

**Headers (by detail):**
- `detail=minimal` → `h = "name|line"`
- `detail=signatures` → `h = "name|line|sig"`
- `detail=full` → `h = "name|line|sig|doc|code"`

Example:
```json
{
  "src/calculator.rs": {
    "h": "name|line|sig",
    "f": "add|10|pub fn add(a: i32, b: i32) -> i32\n...",
    "s": "Calculator|3|",
    "c": ""
  },
  "@": {"t": true}
}
```

**Budgeting:**
- Early estimate-based budget stops adding more files.
- Hard enforcement: drop files; if only one file remains, drop rows until it fits.

---

### 2) `find_usages` (DONE)

**Output (compact):**
```json
{
  "sym": "add",
  "h": "file|line|col|type|context",
  "u": "src/main.rs|42|10|call|let x = add(1,2)\n...",
  "@": {"t": true}
}
```

**Budgeting:**
- Enforces `max_context_lines` during collection (stops collecting once reached).
- Optional `max_tokens` truncates by row count.

---

### 3) `query_pattern` (DONE)

**Output (compact):**
```json
{
  "q": "(function_item name: (identifier) @name)",
  "h": "file|line|col|text",
  "m": "src/calculator.rs|10|5|add\n..."
}
```

Notes:
- Capture maps are intentionally omitted for token efficiency.

---

### 4) `symbol_at_line` (DONE)

**Output (compact):**
```json
{
  "sym": "calculate",
  "kind": "fn",
  "sig": "pub fn calculate(x: i32) -> i32",
  "l": 40,
  "scope": "math::Calculator::calculate"
}
```

---

### 5) `parse_diff` (DONE)

**Output (compact):**
```json
{
  "p": "src/calculator.rs",
  "cmp": "HEAD",
  "h": "type|name|line|change",
  "changes": "fn|add|15|sig_changed: fn(i32,i32)->i64\nfn|multiply|25|added"
}
```

---

### 6) `affected_by_diff` (DONE)

**Output (compact):**
```json
{
  "p": "src/calculator.rs",
  "h": "symbol|change|file|line|risk",
  "affected": "add|sig_changed|src/main.rs|42|high\n..."
}
```

---

## Remaining Tool Migrations (Pending)

### A) `view_code`

**Goal output (per PLAN):**
```json
{
  "p": "src/parser.rs",
  "h": "name|line|sig",
  "f": "parse|10|pub fn parse() -> AST\n...",
  "deps": {
    "src/types.rs": "AST|15|struct AST { ... }\nNode|20|enum Node { ... }"
  }
}
```

**Implementation checklist:**
1. Switch main-file function output to compact header+rows.
2. Implement used-only dependency filtering (extract referenced types, filter deps).
3. Add fallback: if too few filtered types (<3), include top exported types.
4. Update all `tests/parse_file_*`, `tests/token_efficiency_test.rs`, and language tests.

---

### B) `template_context`

**Goal output (per PLAN):**
```json
{
  "tpl": "templates/calculator.html",
  "h": "struct|field|type",
  "ctx": "CalculatorContext|result|i32\nCalculatorContext|history|Vec<Entry>"
}
```

**Implementation checklist:**
1. Convert output to compact rows.
2. Ensure template path and struct-definition paths are relative.
3. Stop using `to_string_pretty` (compact JSON only).
4. Update `tests/askama_template_context_test.rs` and any workflow tests.

---

### C) `type_map`

**Goal output (per PLAN):**
```json
{
  "h": "name|kind|file|line|usage_count",
  "types": "Parser|struct|src/parser.rs|10|42\nAST|struct|src/types.rs|15|38"
}
```

**Implementation checklist:**
1. Replace mixed legacy/new output with a single compact list.
2. Maintain truncation signal (likely `@.t`).
3. Update `tests/type_map_test.rs` to parse the compact list.

---

## Tool Descriptions (Required Updates)

All MCP tool descriptions (in `src/tools.rs`) should be updated to explicitly explain:
- **That the output schema is compact and breaking** (no backward-compatible mode).
- **Abbreviations used** (examples):
  - `h`: header (pipe-delimited column names)
  - `f/s/c`: functions/structs/classes row strings (`code_map`)
  - `u`: usages row string (`find_usages`)
  - `m`: matches row string (`query_pattern`)
  - `p`: file path (relative)
  - `l`: line (1-based)
  - `cmp`: compare target (`parse_diff`)
  - `@.t`: truncation marker
- **How to parse** header+rows and unescape `\\n`, `\\r`, `\\|`, `\\\\`.

This is important for both LLM agents and client tooling so they can reliably decode outputs without guessing.

---

## Migration Notes

- This is a breaking schema change across multiple tools.
- Client parsing should be updated to handle:
  - file-keyed maps (`code_map`)
  - row strings (`u`, `m`, `changes`, `affected`, `types`, etc.)
  - escaping rules for `|` and newlines
