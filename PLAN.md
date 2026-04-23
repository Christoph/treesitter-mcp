# Context Quality Improvement Plan

Updated: 2026-04-22

## Purpose

This MCP server already does two things well:

- extracts useful structure from source with tree-sitter
- delivers that structure in a compact, token-aware format

The next step is to make the context not just smaller, but more reliable --
and to make it work *with* LSP rather than trying to replace it.

### Design position: MCP + LSP, not MCP vs LSP

Tree-sitter gives fast, reliable, language-agnostic syntax. LSP gives
precise, compiler-backed semantics. Trying to build a symbol index from
tree-sitter alone produces a mediocre middle ground: worse resolution than
LSP, more complexity than syntax-aware grep, unstable across languages.

The right split:

| Capability | Owner | Why |
|---|---|---|
| Compact structural overviews | MCP | LSP has no equivalent |
| Token-budgeted context slicing | MCP | LSP returns full text |
| Structural diffs and impact summaries | MCP | LSP has no equivalent |
| Type maps with usage ranking | MCP | LSP doesn't rank by project relevance |
| Scope-qualified syntax search | MCP | Faster than LSP for broad sweeps |
| Precise symbol resolution | LSP | Compiler-backed, handles imports/generics/traits |
| Go-to-definition | LSP | Authoritative |
| Find-references (precise) | LSP | Authoritative |
| Diagnostics, completions | LSP | Compiler-backed |

The MCP should:
- accept LSP results as input where useful (resolved references, definition locations)
- format them compactly for LLM consumption
- provide structural context that LSP cannot
- be honest about confidence when operating without LSP

An LLM with both MCP and LSP should be able to navigate, understand, and
change a codebase primarily through tool responses, using raw file reads
only as a last resort.

## Current Assessment

### Strengths

- Compact row-based output across the main tool surface
- Relative paths and token budgeting treated consistently
- Broad language coverage and healthy test suite
- `cargo test` passes
- `cargo clippy -- -D warnings` passes
- `find_usages` now includes scope, confidence, and owner metadata
- `affected_by_diff` now deprioritizes unrelated same-name symbols
- Main directory scanners now share Git-aware traversal for ignored files
- Type usage ranking separates same-name types by defining file
- `view_code` dependency selection is AST-position-backed for Rust,
  TypeScript, Python, and Go, with no arbitrary dependency padding
- `minimal_edit_context` includes direct project-local dependency signatures
  from imported files
- `call_graph` returns compact best-effort callers/callees with depth and
  budget enforcement
- `format_diagnostics` accepts LSP diagnostics and emits compact rows with
  structural owners
- `view_code` accepts LSP definition locations and narrows dependency
  context to the exact type at that definition
- Strict tiktoken-budget fixtures cover the remaining LSP bridge and
  precision-tool surfaces

### Remaining Shortcomings

No open plan shortcomings remain in this worktree.

## Status Update

Completed in the current worktree:

- **Workstream 0**: repo hygiene and truthfulness work is done; the current
  tree is doc-correct and clippy-clean.
- **Workstream 1**: `find_usages` now emits scope-qualified rows with
  confidence markers, and `affected_by_diff` uses that signal to lower the
  risk of unrelated homonyms.
- **Workstream 4**: `find_usages`, `code_map`, `usage_counter`, and type
  extraction now share Git-aware traversal that skips ignored files in git
  repos, while preserving legacy skip behavior for common build directories.
- **Workstream 2**: `type_map` and `code_map(with_types=true)` now keep
  duplicate type names file-qualified for usage ranking.
- **Workstream 3**: `view_code(include_deps=true)` now selects dependency
  types from AST-position type references for Rust, TypeScript, Python, and
  Go, and returns only explicitly referenced dependency rows.
- **Workstream 5 (phase 1)**: `format_references` accepts LSP-provided
  reference locations and emits the same compact usage schema as
  `find_usages`, with `conf=high`.
- **Workstream 5 (phase 2)**: `view_code(definition_location=...)`
  accepts LSP-provided definition locations and uses them to select the
  exact dependency type.
- **Workstream 5 (phase 3)**: `format_diagnostics` accepts LSP-provided
  diagnostics and emits compact severity/file/owner rows.
- **Workstream 6 (initial)**: `minimal_edit_context` returns target symbol
  code plus same-file callee signatures, same-file referenced types, relevant
  imports, and direct project-local dependency signatures from imported files,
  with a 3x token-savings fixture versus focused `view_code`.
- **Workstream 7 (initial)**: `call_graph` returns compact best-effort
  caller/callee rows for depth 1, supports bounded depth traversal with a
  visited set, and enforces token budgets.
- **Workstream 8**: adversarial fixtures now cover scope
  disambiguation, homonym suppression in `affected_by_diff`, ignored-file
  traversal, duplicate type ranking, AST-backed dependency extraction, and
  LSP reference/definition/diagnostic formatting, focused edit-context
  reduction, call-graph traversal, and strict tiktoken budget enforcement.

## Success Criteria

The work in this plan is done when:

- [x] `cargo test` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] README and tool descriptions do not overclaim semantic guarantees
- [x] same-name types in different files get separate usage counters
- [x] `find_usages` results include scope context so homonyms are distinguishable
- [x] `affected_by_diff` uses scope-qualified/confidence-aware matching and is
  less noisy on rename/signature-change flows
- [x] `view_code` dependency context comes from AST-position type extraction,
  not capitalized-token scanning
- [x] directory traversal respects `.gitignore` in git repos for the main
  directory-scanning tools
- [x] results include confidence markers where resolution is heuristic
- [x] at least one LSP integration point exists (accept resolved references,
  format compactly)
- [x] `minimal_edit_context` returns focused context at least 3x smaller than
  `view_code(focus_symbol=X)` on files with 10+ symbols
- [x] `call_graph` returns correct callers and callees for depth=1
- [x] token budgets remain within current rough envelopes for the remaining
  roadmap items

## Guiding Principles

- Do not reinvent what LSP does; complement it
- Scope-qualified syntax over bare-name matching
- AST-backed signals over string heuristics
- Honest confidence markers over silent guessing
- Precision per token, not precision at any cost
- Independently shippable milestones
- Each improvement testable with targeted fixtures

## Workstreams

### 0. Repo Hygiene and Truthfulness [Complete in current worktree]

Goal: clean, honest state before behavior changes.

Status:

- Completed on 2026-04-20 in the current worktree.

Tasks:

- Remove or wire up dead `code_map` helpers (`build_compact_output`,
  `collect_files`, `process_file`) so `cargo clippy -- -D warnings` passes.
- Fix any broken or accidentally nested tests.
- Change "Semantic search, not text search" to "Syntax-aware search" in
  README and tool descriptions.
- Remove the "semantic search with usage types" claim from `find_usages`
  description.
- Add a short "precision vs heuristic" note to the docs explaining which
  tools provide strong guarantees and which are best-effort.

Acceptance criteria:

- zero-warning clippy run
- no public documentation claiming semantic resolution where only lexical
  or syntax-aware matching exists

### 1. Scope-Qualified Usage Matching [Complete in current worktree]

Goal: make `find_usages` and `affected_by_diff` distinguish homonyms without
building a symbol index.

The key insight: tree-sitter already gives scope context via AST parent
traversal. The infrastructure exists in `symbol_at_line.rs`
(`collect_scope_chain`). Use it.

Status:

- Completed on 2026-04-20 in the current worktree.
- Current compact row shape is:
  `file|line|col|type|context|scope|conf|owner`

#### Phase 1: Add scope context to `find_usages` results

- For each found identifier, walk up the AST to extract the enclosing scope
  chain (function -> class/impl/trait -> module).
- Add `scope` as a column in the output:
  `file|line|col|type|scope|context`
- When a user searches for `add`, results distinguish `Calculator::add` vs
  `math::add` vs local variable `add`.

#### Phase 2: Use scope context in `affected_by_diff`

- When a diff changes `Calculator::add`, search for usages of `add` but
  filter or deprioritize results where the scope chain doesn't match.
- Separate "same text, matching scope" (high confidence) from "same text,
  different scope" (low confidence).

#### Phase 3: Confidence field

- Add a `conf` column or result-level field: `high|medium|low`.
  - `high`: definition site, or usage where scope chain matches
    unambiguously.
  - `medium`: usage in same file/module as a definition.
  - `low`: bare name match across files with no scope alignment.
- `affected_by_diff` uses confidence to weight risk: low-confidence matches
  get `risk=low` regardless of change type.

Tests to add:

- same-name local variable vs function
- top-level function vs class method with same name
- duplicate type names in different modules
- `affected_by_diff` on a rename where unrelated same-name symbols exist

Acceptance criteria:

- `find_usages` output includes scope context
- `affected_by_diff` does not flag unrelated same-name symbols as high risk
- confidence field is present and reflects scope alignment

### 2. File-Qualified Type Ranking [Complete in current worktree]

Goal: stop conflating unrelated types with the same name in `type_map` and
`code_map(with_types=true, count_usages=true)`.

Status:

- Completed on 2026-04-21 in the current worktree.
- `usage_counter.rs` now resolves same-name candidates by defining file,
  same-file references, and project-local dependencies.
- Tests cover duplicate `Config` interfaces in separate TypeScript files for
  both `type_map` and `code_map(with_types=true)`.

Original problem: `usage_counter.rs` keyed counts by bare type name. Two
`Config` structs in different modules shared one counter.

Fix:

- Key usage counts by `(name, file)` instead of just `name`.
- When the same name appears in multiple files, each gets its own counter.
- This is a small change to `count_all_usages` and `apply_usage_counts`.
- No semantic type-reference graph needed.

Optional enhancement:

- Count only references from AST positions (type annotations, field types,
  generic arguments, return types) instead of all word occurrences. This
  is a per-language tree-sitter query, not a symbol index. Implement for
  Rust and TypeScript first; fall back to word counting for other languages.

Tests to add:

- two `Config` structs in different files: separate counters
- two `Error` enums with different consumers: separate counters
- cross-file import that references one but not the other

Acceptance criteria:

- same-name types in different files get separate usage counters
- ranking is deterministic
- performance remains acceptable

### 3. AST-Position Dependency Extraction for `view_code` [Complete in current worktree]

Goal: replace the capitalized-token heuristic in `view_code` dependency
selection with actual type references from AST positions.

Status:

- Completed on 2026-04-21 in the current worktree.
- Rust, TypeScript, Python, and Go use AST-derived shape/type extraction for
  dependency type selection.
- Unsupported languages still use the legacy heuristic fallback.
- Dependency output no longer pads with arbitrary exported types; only
  explicitly referenced dependency rows are returned.
- Go module-local imports from `go.mod` are resolved to project package
  files, and Go dependency output includes both structs and interfaces.

Original problem: `extract_referenced_type_names` in `view_code.rs` scanned
raw text for tokens starting with uppercase. The fallback padded with
arbitrary exported types when fewer than 3 matches were found.

Fix:

- Replace `collect_type_like_tokens` with language-specific tree-sitter
  queries that extract types from:
  - parameter types
  - return types
  - field types
  - generic arguments
  - extends / implements clauses
- Implement for Rust, TypeScript, Python, Go. Each query is ~10-20 lines.
- Fall back to the current heuristic for unsupported languages, but mark
  those results as `fallback`.
- Remove the fallback padding that injects arbitrary exported types. If no
  relevant dependencies are found, return no deps rather than noise.

Tests to add:

- file with many capitalized identifiers in comments/strings: should not
  appear as dependencies
- file mentioning unrelated types by name but not referencing them
  syntactically
- focused read where only one dependency is truly relevant

Acceptance criteria:

- default dependency output contains only AST-referenced types or explicitly
  marked fallback types
- no arbitrary padding with unrelated exports
- focused reads are more on-topic

### 4. `.gitignore`-Aware Directory Traversal [Complete for git repos in current worktree]

Goal: respect `.gitignore` instead of hardcoding skip patterns.

Current problem: `find_usages`, `usage_counter`, and type extraction all
hardcode skip lists (`target`, `node_modules`, `vendor`, `dist`, `build`).
This misses project-specific ignores and processes files that git ignores.

Status:

- Completed on 2026-04-20 for git repos in the current worktree.
- Implemented via a shared project file walker rather than the originally
  proposed `ignore` crate refactor.

Fix:

- Replace hardcoded skip logic with shared traversal that consults Git's
  ignore state in git repos and preserves legacy skip behavior for common
  build directories.
- Apply consistently across `find_usages`, `usage_counter`, type
  extraction, and `code_map`.

Acceptance criteria:

- files matched by `.gitignore` are skipped in git repos
- existing hardcoded skips still work (they're covered by `.gitignore` in
  most projects)

### 5. LSP Integration Points [Complete in current worktree]

Goal: let agents combine LSP precision with MCP compactness.

#### Phase 1: Accept resolved references

- Status: completed on 2026-04-21 in the current worktree.
- Added `format_references`, which accepts either compact 1-based
  `{file,line,col}` / `{file_path,line,column}` locations or LSP
  `{uri,range:{start:{line,character}}}` locations.
- Output uses the native `find_usages` compact schema:
  `file|line|col|type|context|scope|conf|owner`, with `conf=high`.
- Tests cover compact locations and LSP URI/range locations.

Original task:

- Add a tool (or parameter on `find_usages`) that accepts a list of
  locations from LSP `textDocument/references` and formats them in the
  compact MCP schema with scope context and confidence=high.
- This lets an agent: call LSP for precise references, then pass them
  through MCP for compact formatting and risk assessment.

#### Phase 2: Accept definition location

- Status: completed on 2026-04-22 in the current worktree.
- Added `definition_location` on `view_code`, accepting compact
  `{file,line,col}` / `{file_path,line,column}` locations or LSP
  `{uri,range:{start:{line,character}}}` locations.
- When provided, dependency context is narrowed to the type defined at
  that location instead of broad import/name inference.

- Add a parameter on `view_code` that accepts a definition location from
  LSP `textDocument/definition` and uses it to resolve the correct
  dependency type, avoiding the heuristic entirely.

#### Phase 3: Compact LSP diagnostics

- Status: completed on 2026-04-22 in the current worktree.
- Added `format_diagnostics`, which accepts either compact 1-based
  `{file,line,col}` / `{file_path,line,column}` diagnostics or LSP
  `{uri,range:{start:{line,character}}}` diagnostics.
- Output uses compact rows:
  `severity|file|line|col|owner|source|code|message`.
- Tests cover LSP URI/range diagnostics, compact locations, severity
  ordering, structural owner context, and token budget truncation.

- Accept LSP `textDocument/diagnostics` and return a compact, token-
  efficient summary grouped by severity and file, with structural context
  (which function/class owns each diagnostic).

Suggested tool shape:

```
format_references:
  input: [{file, line, col}, ...], symbol_name
  output: compact MCP schema with scope, usage type, confidence=high

format_diagnostics:
  input: [{file, line, severity, message}, ...]
  output: compact grouped summary with structural owners
```

Acceptance criteria:

- at least `format_references` exists and produces compact output from
  LSP-provided locations
- output is indistinguishable in schema from native `find_usages` output
  but with confidence=high

### 6. Minimal Edit Context [Initial version complete in current worktree]

Goal: return the absolute minimum context needed to correctly edit one
symbol, saving 10-40x tokens compared to reading the full file.

Current problem: `view_code(focus_symbol=X)` returns the full file with
other symbols collapsed to signatures. That still includes unrelated
functions, structs, and imports. An agent editing a 20-line function in a
500-line file pays for all 500 lines of structure.

New tool: `minimal_edit_context`

Status:

- Initial version completed on 2026-04-22 in the current worktree, then
  extended with direct project-local dependency signatures.
- Supports locating top-level functions, class methods, and Rust impl
  methods via `extract_enhanced_shape`.
- Returns the target symbol code, same-file callee signatures from AST call
  sites, direct project-local dependency signatures from imported files,
  same-file referenced type rows, relevant import rows, and scope.
- Enforces token budget by dropping optional context before the target.
- Current limitation: dependency signature resolution is direct-import only,
  not transitive or compiler-grade.

Input:

- `file_path`: path to the source file
- `symbol_name`: the symbol to edit

Output:

- `target`: full code of the target symbol
- `deps`: signatures of symbols it calls or references (same file or
  project), extracted from AST call sites and type annotations
- `types`: compact definitions of types referenced in its signature and
  body (parameter types, return type, field types)
- `imports`: only the import lines relevant to this symbol
- `scope`: enclosing scope chain for orientation

Implementation:

1. Parse the file, locate the target symbol.
2. Walk the symbol's AST subtree to collect:
   - identifiers in call expressions (outgoing calls)
   - type identifiers in annotations, parameters, return types
   - identifiers that match import bindings
3. For each collected reference, resolve to a signature within the same
   file or from project dependencies (reuse `view_code` dependency
   infrastructure).
4. Filter imports to only those that bind names used in the target symbol.
5. Assemble into compact output with token budget enforcement.

Relationship to existing tools:

- Reuses `extract_enhanced_shape` for symbol extraction.
- Reuses `resolve_dependencies` and AST-position type extraction from
  Workstream 3 for dependency signatures.
- Reuses scope chain from Workstream 1 for the `scope` field.

Tests to add:

- function that calls 2 of 10 functions in the file: only those 2 appear
  in deps
- function that uses 1 of 5 imported types: only that import appears
- function with no external dependencies: output contains only the target
- compare token count vs `view_code(focus_symbol=X)` on a large file:
  must be materially smaller

Acceptance criteria:

- `minimal_edit_context` returns only context relevant to the target symbol
- token count is at least 3x smaller than `view_code(focus_symbol=X)` on
  files with 10+ symbols
- output includes enough context that an LLM can make a correct edit
  without reading the full file

### 7. Call Graph Extraction [Initial version complete in current worktree]

Goal: answer "what calls this?" and "what does this call?" in one compact
tool call, replacing multi-file reads and manual filtering.

New tool: `call_graph`

Status:

- Initial version completed on 2026-04-22 in the current worktree.
- Returns rows shaped as `direction|symbol|file|line|scope|depth`.
- Resolves project-local definitions best-effort, preferring same-file
  definitions.
- Supports `direction=callers|callees|both`, `depth` up to 3, a visited set
  for recursive graphs, and token budget truncation.
- Tests cover depth-1 callers/callees, depth-2 transitive callees, and
  recursive functions.

Input:

- `file_path`: path to the source file containing the symbol
- `symbol_name`: the function/method to analyze
- `direction`: `callers` | `callees` | `both` (default: `both`)
- `depth`: how many levels to traverse (default: 1, max: 3)
- `max_tokens`: token budget (default: 2000)

Output:

- `sym`: the target symbol
- `h`: header for edge rows
- `edges`: compact rows of call relationships

```
h: "direction|symbol|file|line|scope|depth"
edges: "callee|validate|src/lib.rs|18|process|1\ncaller|main|src/main.rs|42||1"
```

Implementation:

Callees (outgoing edges):

1. Parse the file, locate the target symbol's AST node.
2. Query for `call_expression` and `method_call_expression` nodes within
   the symbol's subtree.
3. Extract the called function/method name from each call site.
4. For each callee, resolve file and line from project-local definitions
   (best-effort: search same file first, then project).
5. Depth > 1: recursively extract callees of callees, up to depth limit.

Callers (incoming edges):

1. Run `find_usages` for the symbol, filtered to `type=call`.
2. For each caller, extract scope chain to identify the enclosing function.
3. Depth > 1: recursively find callers of callers.

Language support:

- Rust: `call_expression`, `method_call_expression`
- TypeScript/JavaScript: `call_expression`, `member_expression` in call
  position
- Python: `call` node type
- Go: `call_expression`

Relationship to existing tools:

- Callers reuse `find_usages` filtered to calls.
- Scope chain reuse from Workstream 1.
- Callee extraction is new AST query work but follows the same pattern
  as type extraction in Workstream 3.

Tests to add:

- function that calls 3 helpers: all 3 appear as callees
- function called from 2 sites: both appear as callers
- depth=2: transitive callees appear with correct depth indicator
- method call on a known type: callee is resolved with scope
- recursive function: does not infinite-loop

Acceptance criteria:

- `call_graph` returns correct callers and callees for depth=1
- depth > 1 works without infinite loops (visited set)
- output stays within token budget
- callers match `find_usages(type=call)` results but with less noise
  (scope-qualified, no non-call usages)

### 8. Targeted Quality Fixtures [Complete in current worktree]

Goal: prove precision improvements with adversarial test cases.

Status:

- Completed on 2026-04-22 in the current worktree.
- Fixtures cover scope qualification, homonym suppression in
  `affected_by_diff`, ignored-file traversal, duplicate type ranking,
  AST-backed dependency extraction, LSP reference formatting, LSP
  definition-location dependency narrowing, LSP diagnostics formatting,
  project-local edit context, call graph traversal, and strict tiktoken
  budget enforcement for the remaining roadmap tools.

Add fixtures alongside each workstream, not as a separate evaluation
framework:

- **Scope qualification**: same-name symbols in different scopes, verify
  scope column distinguishes them.
- **Type ranking**: duplicate-name types in different files, verify
  separate counters.
- **Dependency extraction**: files with misleading capitalized tokens in
  comments, verify AST extraction ignores them.
- **Affected-by-diff**: rename fixture where unrelated same-name symbols
  exist, verify they are not flagged as high risk.

Each fixture should assert both correctness and token efficiency (output
does not grow unreasonably).

Acceptance criteria:

- at least one adversarial fixture per workstream
- quality regressions fail CI

## Execution Order

### Milestone 1: Cleanup and Truthfulness (Workstream 0) [Complete]

- Fix clippy dead code
- Honest docs
- Ship immediately

### Milestone 2: Scope-Qualified Matching (Workstream 1) [Complete]

- Phase 1: scope column in `find_usages`
- Phase 2: scope-qualified `affected_by_diff`
- Phase 3: confidence markers
- Fixtures for scope disambiguation

### Milestone 3: Relevance Upgrades (Workstreams 2 + 3) [Complete]

- File-qualified type ranking
- AST-position dependency extraction
- Fixtures for type ranking and dependency relevance

### Milestone 4: Infrastructure (Workstream 4) [Complete for git repos]

- `.gitignore`-aware traversal across all tools

### Milestone 5: Precision Tools (Workstreams 6 + 7) [Initial complete]

- `minimal_edit_context` tool (depends on Workstreams 1 + 3)
- `call_graph` tool (depends on Workstream 1)
- Fixtures for both

### Milestone 6: LSP Bridge (Workstream 5) [Complete]

- `format_references` tool
- LSP-aware `view_code` dependency resolution
- Compact diagnostics formatting

### Milestone 7: Prove It (Workstream 8) [Complete]

- Adversarial fixtures for all workstreams
- Token efficiency assertions

## What This Plan Does Not Do

- Build a project-wide symbol index from tree-sitter. LSP does this better.
- Attempt compiler-grade name resolution. LSP does this better.
- Replace LSP for precise go-to-definition or find-references.
- Broaden language support before precision improves on existing languages.
- Replace compact schemas with verbose JSON.

## Definition of Done

This plan is complete when:

- the repo is lint-clean
- the docs are honest about what is syntax-aware vs semantic
- `find_usages` returns scope context and confidence
- same-name types get separate usage counters
- `view_code` deps come from AST positions, not capitalized-token scanning
- `affected_by_diff` is less noisy on collision-heavy fixtures
- `minimal_edit_context` exists and produces focused output
- `call_graph` exists and returns callers/callees
- at least one LSP integration point exists
- compact output is preserved
- agents with MCP + LSP can navigate codebases primarily through tool
  responses
