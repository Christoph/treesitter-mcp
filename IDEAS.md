# Improvement Ideas

Updated: 2026-04-17

## Purpose

This file collects ideas that could make `treesitter-mcp` more useful for
coding LLMs beyond the core work in [PLAN.md](./PLAN.md).

The core design tension:

- improve coding capability
- minimize token usage

The ideal outcome: an MCP surface that lets an LLM understand, navigate,
review, and change a codebase while reading as little raw code as possible,
using LSP for precision where available.

## 1. Minimal Edit Context

### Idea

Given a symbol to edit, return the absolute minimum context needed to make
a correct edit: the symbol's code, its signature dependencies, its callers'
expectations, and nothing else.

This is different from `view_code(focus_symbol=X)` which returns the whole
file with other symbols as signatures. Minimal edit context would return
only:

- the target symbol's full code
- signatures of symbols it calls or references
- type definitions for its parameters, return type, and field types
- import context needed to understand the code
- nothing from unrelated parts of the file

```
minimal_edit_context:
  input: file_path, symbol_name
  output:
    target: full code of the symbol
    deps: signatures of referenced symbols (from same file or project)
    types: definitions of referenced types (compact)
    imports: relevant import lines only
```

### Why this matters for LLMs

LLMs waste the most tokens on context they don't need for the current edit.
A 500-line file where the agent needs to change one 20-line function still
costs full-file tokens today. This tool would return ~50-80 tokens of
focused context instead of ~2000.

### Cost / Tradeoffs

- Needs reliable extraction of what a symbol references (tree-sitter can
  handle this for call sites and type annotations).
- Risk of missing context the LLM actually needs. Must be paired with
  confidence markers so the LLM knows when to request more.

## 2. Call Graph Extraction

### Idea

For a given function, extract:

- what it calls (outgoing edges)
- what calls it (incoming edges)
- depth-limited transitive closure in either direction

Return as compact rows: `caller|callee|file|line`

Tree-sitter can extract call sites reliably from AST positions
(`call_expression`, `method_call_expression`). Combining outgoing calls
from AST with incoming calls from `find_usages` (filtered to `type=call`)
gives a practical call graph without LSP.

### Why this matters for LLMs

"What calls this?" and "What does this call?" are the two most common
questions during refactoring. Currently agents either:
- call `find_usages` and manually filter to calls (expensive, noisy)
- read multiple files to trace control flow (very expensive)

A dedicated call graph tool answers both questions in one compact response.

### Potential shape

```
call_graph:
  input: file_path, symbol_name, direction=both|callers|callees, depth=1
  output:
    h: "direction|symbol|file|line|scope"
    edges: "callers|main|src/main.rs|42|main\ncallees|validate|src/lib.rs|18|process"
```

### Cost / Tradeoffs

- Outgoing edges are cheap (AST query on one function).
- Incoming edges require `find_usages` which is a project scan.
- Depth > 1 multiplies cost. Default depth=1, let agent request more.

## 3. Change Impact Preview

### Idea

Given a *planned* change (not yet made), predict what will need to change
downstream. This is `affected_by_diff` but *before* the edit.

Input: a symbol name and a description of the planned change (e.g.,
"add parameter `timeout: Duration` to `fetch_data`").

Output: list of call sites that would need updating, with current code
snippets and suggested scope of change.

Alternatively, simpler version: just accept a new signature string and
diff it against the current signature to produce the impact analysis.

```
preview_impact:
  input: file_path, symbol_name, new_signature
  output: same schema as affected_by_diff but without needing actual changes
```

### Why this matters for LLMs

Agents currently have to: make the change -> run `affected_by_diff` ->
discover it broke 15 call sites -> fix them all. With preview, the agent
can see the blast radius *before* committing to the change, potentially
choosing a less disruptive approach.

### Cost / Tradeoffs

- Essentially a virtual diff: parse the new signature, compare to current,
  run the impact analysis without modifying the file.
- Implementation is mostly reusing `affected_by_diff` internals with a
  synthetic old/new comparison.

## 4. Test Relevance Mapping

### Idea

Given a changed symbol, identify which test files and test functions are
most likely to exercise it.

Approach:
- find test files (files matching `*test*`, `*spec*`, or in `tests/` dirs)
- search for usages of the changed symbol within test files only
- rank by: direct call > transitive reference > same module

```
relevant_tests:
  input: file_path, symbol_name
  output:
    h: "test_file|test_fn|line|relevance"
    tests: "tests/calc_test.rs|test_add|15|direct_call\n..."
```

### Why this matters for LLMs

After making a change, agents often run the entire test suite or guess
which tests to run. A targeted test list saves time and tokens (no need
to read test output for irrelevant tests).

### Cost / Tradeoffs

- Simple implementation: `find_usages` scoped to test directories.
- Heuristic test file detection may miss unconventional test layouts.

## 5. Structural Similarity Search

### Idea

Find functions/methods that are structurally similar to a given one.
Useful for: finding duplicate logic, identifying extraction candidates,
understanding patterns.

Compare by:
- parameter count and types
- return type
- called functions
- AST shape (depth, node type distribution)

### Why this matters for LLMs

When an agent is asked to "add a function like X but for Y", finding
existing similar functions gives a concrete template. When refactoring,
finding near-duplicates identifies extraction opportunities.

### Cost / Tradeoffs

- AST shape comparison is cheap per-function but expensive across a whole
  project. Needs a pre-filtering step (same parameter count, similar size).
- Similarity threshold tuning to avoid noise.

## 6. Progressive Context Disclosure

### Idea

Instead of returning full context in one call, support a "drill-down"
pattern where the first response is extremely compact and the agent
requests more detail on specific items.

Level 0: names and line numbers only (~10 tokens per symbol)
Level 1: add signatures (~30 tokens per symbol)
Level 2: add doc comments (~50 tokens per symbol)
Level 3: full code (~200+ tokens per symbol)

The agent starts at level 0, identifies the 2-3 symbols it cares about,
and requests level 3 only for those.

This is partially supported today via `detail="minimal"|"signatures"|"full"`
and `focus_symbol`, but could be made more systematic with a single
progressive-disclosure API.

### Why this matters for LLMs

Current workflow: `view_code(detail=signatures)` -> identify target ->
`view_code(focus_symbol=X)`. Two tool calls. A progressive API could
combine this into one round-trip with a continuation token.

### Cost / Tradeoffs

- Needs stateful session or continuation tokens (adds complexity).
- May not be worth it if the two-call pattern is fast enough.
- Alternative: just make the two-call pattern cheaper (already mostly is).

## 7. Multi-File Batch Operations

### Idea

Accept multiple file paths or symbol names in a single tool call and
return combined results.

```
batch_view_code:
  input: [{file_path, focus_symbol?}, ...]
  output: combined compact schema, one entry per file

batch_find_usages:
  input: [symbol1, symbol2, ...], path
  output: combined usage rows, grouped by symbol
```

### Why this matters for LLMs

Agents working on multi-file changes currently make N sequential tool
calls. If an agent knows it needs to view 3 files, one batched call saves
2 round-trips and reduces prompt overhead (no repeated instructions).

### Cost / Tradeoffs

- Simple to implement: loop over inputs, merge outputs.
- Token budget needs to be shared across all inputs (sum of individual
  budgets, or a global budget that gets split).
- Risk of returning too much in one response. Needs good truncation.

## 8. Ownership and Module Boundary Detection

### Idea

Detect and expose module/package boundaries so agents understand
architectural structure.

- Rust: crate and module hierarchy from `mod` declarations
- Python: package structure from `__init__.py` files
- TypeScript: barrel exports from `index.ts` files
- Go: package declarations

Output: a tree of modules with their public API surface.

```
module_map:
  input: path
  output:
    h: "module|exports|file"
    modules: "api|[handlers, routes, middleware]|src/api/mod.rs\n..."
```

### Why this matters for LLMs

Agents making changes often violate module boundaries because they don't
see them. A module map tells the agent "this function is internal to this
module, don't call it from outside" without the agent having to infer
visibility rules from code.

### Cost / Tradeoffs

- Language-specific implementation, but mostly parsing import/export
  patterns that tree-sitter handles well.
- Visibility rules vary significantly across languages.

## 9. Compact Contextual Errors

### Idea

When a tool encounters an issue (file not found, parse error, ambiguous
symbol), return structured error context that helps the agent self-correct
instead of a bare error message.

Current: `"Path does not exist: /foo/bar.rs"`
Better:

```json
{
  "error": "file_not_found",
  "path": "/foo/bar.rs",
  "suggestions": ["src/foo/bar.rs", "src/bar.rs"],
  "hint": "did you mean a relative path?"
}
```

For ambiguous symbols:

```json
{
  "error": "ambiguous_symbol",
  "symbol": "Config",
  "candidates": [
    {"file": "src/api/config.rs", "line": 5, "kind": "struct"},
    {"file": "src/db/config.rs", "line": 12, "kind": "struct"}
  ],
  "hint": "specify file_path to disambiguate"
}
```

### Why this matters for LLMs

Bare error messages cause agents to retry blindly or ask the user. Rich
error context lets agents self-correct in one step: "file not found, but
similar file exists at X, retrying with X".

### Cost / Tradeoffs

- Needs fuzzy path matching for suggestions (cheap with `walkdir`).
- Slightly larger error responses, but they replace a retry round-trip.

## 10. Diff-Scoped Context Assembly

### Idea

Given a git diff (or a set of changed files), automatically assemble the
minimum context an agent needs to review the change:

- changed symbols (from `parse_diff`)
- signatures of affected callers (from `affected_by_diff`)
- type definitions referenced by changed signatures
- test functions that exercise changed code

One tool call that answers: "here is everything you need to understand and
review this change."

```
review_context:
  input: compare_to="HEAD~1", scope="src/"
  output:
    changes: compact structural diff
    affected: risk-sorted usage rows
    types: referenced type definitions
    tests: relevant test functions
    tokens_saved: estimate vs reading all changed files
```

### Why this matters for LLMs

Code review is the highest-value use case for coding LLMs, and it's
currently the most token-expensive. An agent reviewing a PR typically
reads every changed file in full. A `review_context` tool could reduce
that to 20-30% of the tokens while preserving all the signal.

### Cost / Tradeoffs

- Composition of existing tools, but the assembly logic needs to be smart
  about what to include and what to skip.
- Token budget allocation across the sub-components is tricky.

## 11. Symbol Rename Dry-Run

### Idea

Given a symbol and a new name, show exactly what would change across the
project without making any edits:

- all usage locations
- the edits that would be applied at each location
- confidence that each edit is correct
- files that would be modified

```
rename_preview:
  input: file_path, symbol_name, new_name
  output:
    h: "file|line|col|old_text|new_text|confidence"
    edits: "src/main.rs|42|10|add(|add_numbers(|high\n..."
    files_modified: 3
    total_edits: 7
```

### Why this matters for LLMs

Rename is the most common refactoring operation and the one most likely
to cause subtle breakage. LSP can do precise renames, but this tool
provides a preview that the agent (and user) can review before applying.
Combined with LSP rename, it gives a verify-then-apply workflow.

### Cost / Tradeoffs

- Essentially `find_usages` with edit preview formatting.
- Confidence comes from scope-qualified matching (Workstream 1 in PLAN.md).
- Without LSP, can't guarantee correctness for imports, re-exports, or
  string references.

## 12. Context Budget Advisor

### Idea

A meta-tool that, given a task description and a token budget, recommends
which tools to call and with what parameters to stay within budget.

```
plan_context:
  input: task="rename calculate to compute in src/math/", budget=3000
  output:
    steps:
      - find_usages(symbol="calculate", path="src/math/", max_tokens=1000)
      - view_code(file_path="src/math/calc.rs", focus_symbol="calculate")
    estimated_tokens: 2400
    note: "budget allows full context; no truncation needed"
```

### Why this matters for LLMs

Agent frameworks often have fixed context windows. A budget advisor helps
agents make optimal tool call sequences instead of over-fetching or
under-fetching context.

### Cost / Tradeoffs

- Needs token estimation for each tool (already exists via tiktoken).
- Task understanding is hard; may need to be a simple heuristic rather
  than full planning.
- Risk of the meta-tool itself costing tokens. Keep output minimal.

## 13. Structural Invariant Checking

### Idea

After an edit, verify that the file still parses correctly and that
structural invariants hold:

- all functions that existed before still exist (unless intentionally removed)
- no new parse errors
- signature contracts are maintained (parameter count, return type present)
- import statements are consistent with usage

```
verify_edit:
  input: file_path, compare_to="HEAD"
  output:
    parses: true
    issues: []
    # or
    parses: true
    issues: ["function 'validate' lost return type annotation"]
```

### Why this matters for LLMs

LLMs make subtle structural errors: accidentally deleting a function while
editing a neighbor, dropping return type annotations, breaking indentation
in Python. A fast structural check catches these before the agent moves on.

### Cost / Tradeoffs

- Cheap: just parse old and new, compare symbol sets.
- Limited to structural issues; can't catch logic errors.
- Complements (but doesn't replace) running tests or LSP diagnostics.

## How To Use This File

Use this file for:

- strategic ideas and future directions
- optional enhancements beyond PLAN.md
- evaluation of new tool concepts

Use [PLAN.md](./PLAN.md) for:

- committed work with milestones
- acceptance criteria
- execution order
