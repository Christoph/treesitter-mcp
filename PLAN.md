# Context Quality Improvement Plan

Updated: 2026-04-17

## Purpose

The repo already does two things well:

- it extracts useful structure from source with tree-sitter
- it delivers that structure in a compact, token-aware format

The next step is to make the context not just smaller, but more reliable.
This plan focuses on the gaps found in review: lexical matching where semantic
signals are claimed, weak relevance ranking, noisy dependency expansion, and
some repository hygiene drift.

The product direction is stricter than "better code analysis":

- an LLM should be able to understand and navigate a codebase primarily through
  this MCP surface
- the server should behave like a precision knife, returning the smallest useful
  slice of structure, ownership, impact, and code
- large raw file reads, huge git diffs, and broad context dumps should be the
  fallback path rather than the normal path

## What "better context" should mean

For this project, "better context" should mean:

1. The tool returns the right symbol or type more often when names collide.
2. The tool is explicit when it is guessing.
3. Relevance ranking prefers semantically related context over cheap lexical hits.
4. Compact encoding remains a feature, not the main goal.
5. Outputs stay deterministic and easy for LLMs and clients to parse.
6. Agents can navigate most codebases without needing large unstructured file
   reads or repository-wide diffs.
7. Every precision improvement is evaluated against token cost, with a bias
   toward the smallest output that still preserves the right answer.

## Current Assessment

### Strengths

- Compact row-based output is already in place across the main tool surface.
- Relative paths and token budgeting are treated consistently.
- The repo has broad language coverage and a healthy test suite.
- `cargo test` currently passes.

### Current Shortcomings

1. `find_usages` is lexical, not symbol-aware.
2. Type usage ranking is keyed by bare name, so unrelated types with the same
   name are merged.
3. `view_code` dependency selection uses weak heuristics and fallback padding,
   which can inject irrelevant context.
4. `cargo clippy -- -D warnings` fails because migration leftovers remain.
5. Tests currently prove format stability and token savings better than they
   prove semantic precision.

## Success Criteria

The work in this plan is done when the repo satisfies all of the following:

- `cargo test` passes
- `cargo clippy -- -D warnings` passes
- README and tool descriptions do not overclaim semantic guarantees
- duplicate-name and overload fixtures are handled correctly or marked
  explicitly ambiguous
- `affected_by_diff` is materially less noisy on rename/signature-change flows
- `type_map` and `code_map(with_types=true,count_usages=true)` stop conflating
  unrelated types with the same name
- `view_code` dependency context is relevant by default and does not inject
  arbitrary filler types
- token budgets remain within current rough envelopes for the default modes
- common navigation and review tasks can be completed through MCP responses
  without relying on large raw file dumps or broad git diffs

## Guiding Principles

- Precision before compression
- Precision per token, not precision at any cost
- AST-backed signals before string heuristics
- Language-specific queries where they materially improve quality
- Deterministic output over clever but unstable ranking
- Explicit fallback paths with confidence/ambiguity markers
- MCP-first navigation over raw file reads
- narrow slices over large diffs

## Workstreams

### 0. Repo Hygiene And Truthfulness

Goal: get the repo back into a clean, honest state before deeper behavior work.

Tasks:

- Remove or wire up dead `code_map` helpers so `cargo clippy -- -D warnings`
  passes again.
- Fix broken or accidentally nested tests, especially in
  `tests/askama_template_context_test.rs`.
- Audit README and tool descriptions for claims like "semantic search" and
  either implement them or downgrade the wording until they are true.
- Add a short "precision vs heuristic" note to the docs so users know which
  tools are strong guarantees and which are best-effort.

Acceptance criteria:

- zero-warning clippy run
- no inert tests hidden inside other tests
- no public documentation claiming semantic resolution where only lexical
  matching exists

### 1. `find_usages` And `affected_by_diff`: Move From Lexical To Symbol-Aware

Goal: make rename/refactor workflows trustworthy enough to guide LLM edits.

#### Phase 1: Stop Overclaiming, Add Guardrails

- Reclassify the current implementation as heuristic in docs and descriptions.
- Add ambiguity metadata to results where the engine cannot disambiguate.
- Consider adding a compact confidence field for each row or at the result
  level, for example `conf=high|medium|low`.
- Ensure `affected_by_diff` can skip or down-rank low-confidence hits instead
  of treating them as equally risky.

#### Phase 2: Build A Symbol Index

- For each supported language, extract definitions into a project-local index:
  - name
  - kind
  - file
  - line
  - enclosing scope / owner
  - import/export identity where available
  - signature anchor for overloaded members where the grammar supports it
- Introduce an internal symbol identity instead of bare-name matching.

Suggested shape:

```text
symbol_id := language + file + owner_path + kind + name + signature_key?
```

#### Phase 3: Resolve References Against The Index

- Use AST positions to classify:
  - definition
  - call
  - type reference
  - import/export reference
  - value reference
- Match references against in-scope symbols instead of any identifier with the
  same text.
- Where a language cannot support full resolution cheaply, return an explicit
  ambiguous result instead of pretending the match is definitive.

#### Phase 4: Improve `affected_by_diff`

- Feed it resolved symbol identities instead of bare names.
- When a diff changes a method, search usages of that specific owner+method pair.
- Separate "same text, likely unrelated" from "definitely affected".
- Include risk logic that uses confidence as an input, not just change type.

Tests to add:

- same-name local variable vs function
- top-level function vs class method with same name
- duplicate type names in different modules
- import aliasing
- overloaded methods where supported by the language
- same symbol text used as both value and type

Acceptance criteria:

- new fixtures prove that unrelated same-name symbols are not merged together
- `affected_by_diff` precision improves on targeted rename/signature fixtures
- output stays compact

### 2. Semantic Type Ranking For `type_map` And `code_map`

Goal: rank types by actual project relevance, not bare word frequency.

Current issue:

- usage counts are derived from stripped source text and keyed only by type name
- this conflates unrelated `Config`, `Error`, `Options`, `Manager`, and similar
  names across files and modules

Plan:

- Replace global lexical word counts with a semantic type-reference graph.
- Count references from AST positions such as:
  - type annotations
  - field types
  - generic arguments
  - return types
  - inheritance / implementation clauses
  - constructor / literal sites where they are syntactically clear
- Track counts by resolved definition identity, not just by type name.

Ranking ideas:

- primary score: resolved reference count
- tie-breaker: number of distinct files referencing the type
- optional small bonus for public/exported types
- optional penalty for unresolved lexical-only references

Compatibility:

- keep `usage_count` in the output for schema stability
- change its meaning from lexical hits to resolved references
- if needed, add a future optional metadata field describing the ranking mode

Tests to add:

- two `Config` structs in different modules
- two `Error` enums with different consumers
- generic wrapper types and nested references
- cross-file imports that should rank one type but not its same-name sibling

Acceptance criteria:

- duplicate-name fixtures no longer share one combined counter
- ranking is deterministic and explainable
- performance remains acceptable for medium-size projects

### 3. `view_code` Dependency Context: Prefer Relevant Over Merely Available

Goal: keep dependency context useful during focused reads without polluting the
output with unrelated types.

Current issue:

- dependency relevance comes from capitalized-token scanning over raw source
- when too few hits are found, the tool pads output with early exported types

Plan:

- Replace token scanning with AST-derived type extraction from:
  - parameter types
  - return types
  - field and property types
  - extends / implements clauses
  - generic arguments
  - typed constructor or literal sites where supported
- Build dependency candidates from import resolution plus project-local
  dependency analysis, not raw text alone.
- Remove default fallback padding with arbitrary exported types.
- If fallback remains useful, gate it behind an explicit mode rather than
  default behavior.

Suggested modes:

- `deps_mode=strict`: only resolved or high-confidence referenced types
- `deps_mode=related`: include a small number of nearby related exports when
  strict mode yields nothing
- default should bias toward `strict`

Potential output improvements:

- compact dependency metadata for why a type was included:
  - `imported`
  - `signature_ref`
  - `field_ref`
  - `implements_ref`
  - `fallback`

Tests to add:

- file with many capitalized identifiers in comments or strings
- file mentioning unrelated types by name but not actually referencing them
- focused read where only one dependency is truly relevant
- project with several exports where fallback previously introduced noise

Acceptance criteria:

- default dependency output contains only relevant or explicitly marked fallback
  types
- focused reads are smaller or equally sized and more on-topic

### 4. Evaluation Harness For Context Quality

Goal: stop validating mainly formatting and token size; start validating signal
quality.

Add a dedicated evaluation suite:

- `semantic_usage_precision` fixtures
- `duplicate_name_ranking` fixtures
- `dependency_relevance` fixtures
- `workflow_noise` fixtures for rename and focused-read workflows

Metrics to track:

- precision for `find_usages`
- ambiguity rate for unresolved cases
- false-positive rate for `affected_by_diff`
- ranking quality for top N results in `type_map`
- dependency precision for `view_code`
- token size before/after each improvement

Suggested process:

1. Add golden fixtures with expected compact output rows.
2. Add scenario-level assertions, not just "row count > 0".
3. Keep token-efficiency tests, but pair them with quality assertions.

Acceptance criteria:

- at least one adversarial fixture per supported language family
- quality regressions fail CI

### 5. MCP-First Navigation And Precision Slicing

Goal: make the server sufficient for understanding and navigating a codebase
without forcing the LLM to fall back to large file reads or broad git diffs.

Plan:

- audit the existing tool surface against common agent tasks:
  - "what owns this line?"
  - "what are the important types here?"
  - "what changed structurally?"
  - "what depends on this?"
  - "what is the minimum code I need to inspect next?"
- prefer adding small, high-precision outputs over returning more raw code
- ensure diff-related flows return symbol-level and impact-level slices before
  any line-level or repository-level output
- define a "last resort" policy for when larger context is allowed

Potential acceptance tests:

- an agent identifies the right function to edit in a large file without reading
  the full file
- an agent reviews a signature change using `parse_diff` and
  `affected_by_diff` without needing a full git diff
- an agent understands a multi-file feature from `code_map`, `type_map`,
  `symbol_at_line`, `view_code`, and focused dependency context alone

Acceptance criteria:

- common navigation tasks have an MCP-first workflow documented and tested
- diff-related workflows stay symbol-first by default
- token use for navigation remains materially below raw-file and raw-diff
  fallbacks

## Recommended Execution Order

### Milestone 1: Cleanup And Truthfulness

- workstream 0
- doc wording corrections
- clippy cleanup

### Milestone 2: Safer Symbol Workflows

- workstream 1 phase 1
- workstream 1 phase 2 groundwork
- new fixtures for ambiguity and same-name collisions

### Milestone 3: Relevance Upgrades

- workstream 2
- workstream 3

### Milestone 4: Prove It

- workstream 4
- quality metrics in CI

### Milestone 5: MCP-First Agent Workflows

- workstream 5
- document and benchmark MCP-only navigation flows

## Additional Ideas Beyond The Review Findings

These are not blockers, but they would make the project stronger.

### A. Add Confidence And Ambiguity To The Compact Schemas

LLMs do better when the tool says "I am unsure" instead of returning confident
noise. A small compact field is cheap and valuable.

Examples:

- result-level: `{"conf":"low"}`
- row-level column: `...|confidence`
- ambiguity marker: `{"amb":true}`

### B. Offer Explicit Precision Modes

Not every user needs the same trade-off.

Suggested modes:

- `fast`: current heuristic-heavy behavior with tight token budget
- `balanced`: AST-backed relevance where available
- `strict`: only high-confidence symbol/type matches

### C. Add Symbol Provenance

For top-level outputs, consider compact provenance fields such as owner scope or
module path. That would help LLMs distinguish homonyms without reading more
files.

Examples:

- `owner`
- `module`
- `def_path`

### D. Language Capability Matrix

Some languages will support stronger resolution than others. Document that
honestly so the tool can expose capabilities instead of pretending everything is
equally precise.

Suggested matrix:

- definition extraction quality
- usage resolution quality
- type extraction quality
- dependency resolution quality

### E. Add Workflow-Level Tools Built On The Better Core

Once symbol identity is stronger, the repo could support higher-level tools that
are directly useful to coding LLMs:

- `related_symbols`
- `focused_slice`
- `implementation_summary`
- `safe_rename_preview`

### F. Evaluate Context Quality Against Real Agent Tasks

Create a small benchmark of tasks like:

- rename a method safely
- update a type used across modules
- modify a template context struct
- inspect a large file and identify the right dependency

Measure:

- correctness of the downstream edit
- tool-call count
- token cost
- rate of unnecessary file reads

## Non-Goals For The First Iteration

- full compiler-grade name resolution for every supported language
- perfect interprocedural analysis
- replacing compact schemas with verbose JSON
- broadening language support before precision improves on existing languages

## Definition Of Done

This plan is complete when:

- the repo is lint-clean
- the docs are honest
- semantic precision tests exist and pass
- the main tools are materially less noisy on collision-heavy fixtures
- compact output is preserved
- the project can credibly claim it gives LLMs cleaner context, not only
  smaller context
