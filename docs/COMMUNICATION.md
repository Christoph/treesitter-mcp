# Communication and Token Strategy

This project already has strong token-efficiency mechanics. The gap versus projects like `rtk` is mostly in packaging and repetition: the value proposition needs to be obvious on the first screen, and the pipeline needs to keep proving it.

## Positioning

The clearest message is:

`treesitter-mcp` is a context compressor for code workflows.

More specifically:

- It replaces raw file reads with AST-derived structure.
- It replaces vague exploration with task-specific payloads.
- It keeps outputs bounded with explicit token budgets.
- It gives agents higher-signal context, not just less context.

## What Works in RTK's Communication

The `rtk` README is effective because it does four things immediately:

1. It states the value in one sentence.
2. It shows concrete savings with recognizable workflows.
3. It explains the mechanism in a small number of repeatable strategies.
4. It makes adoption obvious with a short quick start.

This repository should keep using the same communication pattern, adapted to MCP:

1. Lead with measured workflow savings.
2. Describe the product as a context compressor, not just a parser.
3. Show "use this instead of raw reads" guidance early.
4. Keep benchmark proof visible in CI and release preparation.

## Messaging Pillars

Use these three ideas consistently:

- **Smaller**: the payload is materially smaller than raw code reads.
- **Sharper**: the payload is shaped around the task, not around the file system.
- **Safer**: the payload stays bounded and testable under explicit budgets.

## Product Opportunities to Reduce Token Load Further

These are the strongest next candidates.

### 1. Symbol Summary Mode

Add a very small mode for "what does this symbol do?" that returns:

- name
- signature
- enclosing scope
- 1-3 direct callees
- 1-2 line AST-derived summary

This would cover a large share of orientation questions without paying for full code or even `minimal_edit_context`.

### 2. Grouped `find_usages`

Today `find_usages` is already compact, but repeated file paths and owner names still cost tokens on hot symbols. Add an optional grouped mode that emits:

- per-file counts
- per-owner counts
- only the first N detailed rows per group

That would mimic RTK's "group first, expand only where needed" approach.

### 3. Progressive `review_context`

`review_context` currently composes high-value tools, but it still ships all sections in one shape. Add `detail="summary" | "standard" | "deep"` so the default review path can start with:

- structural changes
- top affected usages
- top relevant tests
- changed symbol names only

Then let the agent request focused symbol context separately.

### 4. Better Last-Mile Truncation for Targeted Tools

`minimal_edit_context` now trims rows progressively before dropping whole sections. The same pattern should be applied anywhere the tool still falls from "full section" to "no section" too quickly.

Good candidates:

- `review_context`
- `code_map` type sections
- `format_diagnostics`

### 5. Path Dictionary Compression for Large Outputs

For large multi-file outputs, repeated relative paths can dominate rows. A compact optional mode could emit:

- `files`: `id|path`
- main row tables referencing file ids

This would help `find_usages`, `format_diagnostics`, and `affected_by_diff` in big repos.

### 6. Session Bootstrap Bundle

Add a single "bootstrap this repo" tool or recipe that returns:

- high-usage types
- code map for key directories
- likely entrypoints
- test directories

The goal is to remove the extra tool-selection overhead at the start of a session.

### 7. Changed-Symbol-First Review Digests

For diffs, add a very small digest mode that answers:

- what changed
- what is risky
- which tests matter
- which symbols deserve a deep read

That gives an agent a review-quality overview before paying for nested context payloads.

### 8. Heuristic Smart Read

RTK's `smart` command succeeds because it satisfies "tell me what matters" cheaply. A comparable tool here could return:

- file purpose
- key symbols
- likely edit points
- notable dependencies

This would sit between `code_map` and `view_code`.

## Pipeline Expectations

The pipeline should keep communication honest:

- formatting, tests, and clippy stay mandatory
- token-efficiency threshold tests stay mandatory
- benchmark summaries should be rendered in the GitHub Actions job summary
- release prep should use the same benchmark command as the README refresh flow

## Local Commands

Use these locally when refreshing the proof points:

```bash
cargo test
cargo clippy -- -D warnings
cargo test report_average_token_benchmarks -- --ignored --nocapture
```

The benchmark reporter is intentionally simple so it can feed the README, CI summaries, and release notes without a second implementation.
