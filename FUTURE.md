# FUTURE.md

Roadmap of improvements that would turn `treesitter-mcp` from a "context compressor" into
a tool that measurably improves coding-agent workflows — not just their token bills.

Items are ordered roughly by expected impact-per-effort. Each entry has a **Why** (what
the user pain is), a **Change** (what to build), and a **Proof** (how we would know it
worked).

---

## Tier 1 — High leverage, mostly local changes

### 1. Parse cache with mtime invalidation

- **Why:** README currently states trees are not cached between requests. Every tool
  call re-parses, which dominates latency on multi-tool sessions over medium repos.
- **Change:** Process-lifetime cache keyed by `(canonical_path, mtime, size, language)`.
  On cache hit, reuse the `tree_sitter::Tree` and its derived shape. Invalidate by mtime
  on access; no filesystem watching required.
- **Proof:** Benchmark "10 tool calls over the same repo" before and after. Expect 3–10×
  latency drop on the second call onward. Add a regression test that asserts the cache
  hit counter is non-zero for a repeated call.

### 2. `apply_symbol_edit` tool

- **Why:** Agents currently edit by rewriting file content or by unified diff. Both are
  brittle: whitespace drift, wrong-hunk application, and token-expensive retries. MCP
  already knows each symbol's exact AST span.
- **Change:** New tool `apply_symbol_edit(file_path, symbol_name, new_body, new_signature?)`.
  Splices the replacement at the AST node's byte range, preserving leading comments and
  indentation. Returns a structural diff summary.
- **Proof:** Track edit-success rate on a small corpus of refactor tasks. Fewer failed
  diffs, fewer agent retries, and lower total tokens per completed edit.

### 3. Next-step hints in every response

- **Why:** Agents waste calls choosing the wrong follow-up tool. A one-line hint per
  response can steer the next call cheaply.
- **Change:** Add an optional `hint` field to every tool response with a short
  suggested follow-up, e.g. `"callers span 3 files; try minimal_edit_context on
  build_report for a focused edit"`. Costs ~10–20 tokens per response.
- **Proof:** A/B test on a scripted agent: measure "wrong tool chosen next" rate with
  and without hints. Goal: halve the wasted-call rate.

### 4. Persistent symbol index

- **Why:** `find_usages` and `call_graph` walk files every call. On big repos this is
  the dominant cost. An index makes them O(hits), not O(files).
- **Change:** On first query against a root, build a SQLite or on-disk index of
  `(symbol_name, file, line, kind, scope, hash)`. Update per-file on mtime change.
- **Proof:** Latency benchmark on a 500-file fixture. Expect first call unchanged, later
  calls ≥ 5× faster.

### 5. `estimate=true` cost preview

- **Why:** Agents cannot currently budget before calling. They pay the full cost, then
  discover it exceeded the context window.
- **Change:** Accept `estimate=true` on expensive tools (`find_usages`, `code_map`,
  `review_context`). Return only `{estimated_tokens, estimated_rows, scope_summary}`
  without building the payload.
- **Proof:** Add a workflow recipe to the README. Show an agent using estimate → budget
  → real call, and the total tokens consumed vs a naive call.

---

## Tier 2 — Quality and correctness

### 6. LSP passthrough / hybrid precision

- **Why:** README.md acknowledges that `find_usages`, `call_graph`, and friends are
  syntax-aware, not compiler-grade. Agents that care about renaming or signature changes
  need precise references.
- **Change:** Accept an optional LSP socket in the server config. When present, route
  `find_usages` → `textDocument/references`, `call_graph` → `callHierarchy`, and the
  `definition_location` field → `textDocument/definition`. Fall back to tree-sitter when
  the LSP is unavailable or slower than a timeout.
- **Proof:** Add a correctness benchmark against a repo with trait/impl resolution. Show
  the hybrid mode matching LSP on recall while still being smaller in token output.

### 7. Confidence column surfaced consistently

- **Why:** `find_usages` already carries a `conf` column. Other tools do not. Agents
  can't escalate selectively without a uniform signal.
- **Change:** Add `conf` (high | medium | low) to `call_graph`, `affected_by_diff`,
  `minimal_edit_context`. Document the agent pattern "act on `high`, escalate to LSP
  only when `conf=medium` and the edit is risky".
- **Proof:** Update the Quick Selection Guide to show the escalation pattern. Measure
  whether an agent makes fewer LSP calls while keeping edit accuracy.

### 8. Diff-aware automatic focus

- **Why:** During review or post-edit verification, the agent almost always wants the
  changed symbols plus their direct neighbours. Today that takes two calls.
- **Change:** `view_code` and `code_map` accept `diff_aware=true`. If the git working
  tree has changes, they auto-focus on touched symbols and their immediate callers.
- **Proof:** Recipe in README. Benchmark "post-edit review" workflow end-to-end.

### 9. Dedup / collapse identical hits

- **Why:** `find_usages` rows for the same identifier in nested scopes sometimes appear
  twice (e.g. closure inside a function). Agents get noise.
- **Change:** Collapse rows that share a byte span or tight AST ancestor. Emit a `count`
  column for the collapsed hit.
- **Proof:** Snapshot tests on fixtures known to produce duplicates.

---

## Tier 3 — Reach and packaging

### 10. Session bootstrap tool

- **Why:** Every new session reaches for 3–4 orientation tools. A single call would
  replace that and reduce tool-selection overhead.
- **Change:** `session_bootstrap(path)` returns top-usage types, a minimal `code_map`,
  likely entry points (heuristic `main`, `lib.rs`, `index.ts`), and test directories,
  with a uniform token budget.
- **Proof:** Recipe in README. Log how many bootstrap-alternative tool calls an agent
  skips when it has this.

### 11. Path dictionary compression

- **Why:** In big repos, repeated relative paths dominate `find_usages` and
  `affected_by_diff` rows.
- **Change:** Optional `compact_paths=true` emits a `files: id|path` header and rewrites
  rows to reference file ids.
- **Proof:** Benchmark on a 1k-file fixture. Expect ≥ 20% shrink on hot-symbol searches.

### 12. Streaming / cursor pagination

- **Why:** Some outputs (huge `code_map`, high-fanout `find_usages`) blow past token
  budgets today. `max_tokens` truncates silently.
- **Change:** Accept `cursor` and return one page plus `next_cursor`. Agent pulls
  further pages only if it needs them.
- **Proof:** Add a recipe that demonstrates an agent processing a large search in
  bounded pages without losing hits.

### 13. Copy-paste agent prompt snippet

- **Why:** Users configure the server, then the agent picks the wrong tool. The Quick
  Selection Guide in the README is long and not intended as a system-prompt fragment.
- **Change:** Ship a short (~300-token) snippet designed to be pasted into an agent
  system prompt. Focused on "when to reach for which tool" decision rules.
- **Proof:** Before/after tool-choice correctness on a small scripted benchmark.

---

## Tier 4 — Honest accounting

### 14. Accuracy benchmark harness

See [BENCHMARK.md](BENCHMARK.md) for the full plan. The short version: token savings
alone do not prove a quality win. Until we have a task-success benchmark, the
efficiency claims stand on shaky ground.

### 15. CI token-budget dashboard

- **Why:** CI already fails on payload-size regressions, but users cannot see trends.
- **Change:** GitHub Actions job summary renders the benchmark table as markdown.
  Store results in `benches/history.json` and post a "Δ vs main" diff on pull requests.
- **Proof:** Self-evident in PR UX.

---

## Items explicitly deferred

- Graph algorithms beyond depth-1 callers. Valuable, but `call_graph` at `depth>=2`
  already risks resolving into the wrong trait impl without an LSP. Fix Tier 2 first.
- Embeddings / semantic search. Scope creep, different failure modes, and not what this
  server is positioned for. Keep the product "AST + compact schemas".
