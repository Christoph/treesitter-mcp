# Improvement Ideas

Updated: 2026-04-17

## Purpose

This file collects ideas that could make `treesitter-mcp` more useful for
coding LLMs beyond the core remediation work in [PLAN.md](./PLAN.md).

These are not all immediate commitments. The goal is to capture directions that
could improve context quality, trustworthiness, and downstream agent outcomes.

The core design tension for this project is now explicit:

- improve coding capability
- minimize token usage

The ideal outcome is not "more analysis output". It is an MCP surface that lets
an LLM understand, navigate, review, and change a codebase while reading as
little raw code as possible.

## 1. Confidence And Ambiguity Markers

### Idea

Add explicit confidence or ambiguity signals to tool outputs when symbol or type
resolution is heuristic rather than definitive.

Possible shapes:

- result-level metadata: `{"conf":"low"}`
- row-level column: `...|confidence`
- ambiguity marker: `{"amb":true}`

### Expected Benefits

- Makes the tool more honest. An LLM can treat low-confidence results as hints
  instead of facts.
- Reduces bad follow-on edits caused by confidently presented noise.
- Helps clients and higher-level tools decide whether to ask follow-up
  questions, read more files, or switch to a stricter mode.
- Improves `affected_by_diff` risk scoring, because low-confidence matches can
  be down-ranked or separated from likely breakage.

### Cost / Tradeoffs

- Slightly wider schemas.
- Some client prompts and parsers may need updates to make use of the metadata.

## 2. Precision Modes

### Idea

Expose explicit trade-off modes so users can choose between speed and precision.

Suggested modes:

- `fast`: cheap heuristics, low latency, optimized for exploration
- `balanced`: AST-backed relevance and moderate validation
- `strict`: only high-confidence matches, even if coverage is lower

### Expected Benefits

- Prevents one default from trying to satisfy incompatible use cases.
- Keeps lightweight browsing cheap while giving refactor workflows a safer mode.
- Makes performance expectations easier to document and test.
- Lets downstream agents select the right behavior for the task:
  - exploration
  - debugging
  - rename/refactor
  - impact analysis

### Cost / Tradeoffs

- More testing matrix surface.
- Tool descriptions and examples need to explain the modes clearly.

## 3. Symbol Provenance In Compact Outputs

### Idea

Attach lightweight provenance to symbols and types so homonyms can be
distinguished without opening more files.

Examples:

- owner scope
- module path
- import source
- definition path
- enclosing type or trait

### Expected Benefits

- Helps LLMs distinguish `Config` from `Config`, `add` from `Calculator::add`,
  and similar same-name cases.
- Improves ranking and grouping without needing a full verbose payload.
- Makes outputs more composable across tools, because `view_code`,
  `find_usages`, `type_map`, and `affected_by_diff` could speak about the same
  symbol identity more consistently.
- Lowers the number of follow-up reads required to understand what a result
  actually refers to.

### Cost / Tradeoffs

- Slight token overhead.
- Needs a stable compact representation to avoid creating another parsing burden.

## 4. Language Capability Matrix

### Idea

Document and possibly expose which languages support which quality level for:

- definition extraction
- usage resolution
- type extraction
- dependency relevance
- diff impact analysis

### Expected Benefits

- Keeps the repo honest about where the strongest guarantees exist.
- Helps prioritize engineering effort on the weakest language paths first.
- Gives agent authors a principled way to adjust prompts and fallback behavior.
- Makes regressions easier to spot because improvements can be tracked per
  language instead of only globally.

### Cost / Tradeoffs

- Requires ongoing maintenance as capabilities improve.
- Some users may initially see weaker support than they assumed, but that is
  better than silent overclaiming.

## 5. Higher-Level Workflow Tools

### Idea

Once the symbol and relevance core improves, add tools that map more directly to
what coding LLMs actually need.

Examples:

- `related_symbols`
- `focused_slice`
- `implementation_summary`
- `safe_rename_preview`
- `relevant_types_for_symbol`

### Expected Benefits

- Reduces multi-step tool choreography in prompts.
- Lets the server combine several internal analyses into one better-targeted
  response instead of forcing the LLM to stitch them together.
- Lowers token waste by returning only the information needed for a specific
  coding action.
- Makes the repo feel less like a bag of primitives and more like an LLM-native
  coding context engine.

### Cost / Tradeoffs

- Higher design burden: these tools need strong contracts to avoid becoming
  vague convenience wrappers.
- They should be built on top of stronger symbol identity first, otherwise they
  risk packaging the same noise in a nicer API.

## 6. Real Agent Task Benchmarks

### Idea

Evaluate the repo on realistic coding-agent tasks instead of mainly on output
format and token size.

Possible scenarios:

- rename a method safely
- update a type used across modules
- inspect a large file and choose the right dependency
- modify an Askama template with the correct context struct
- estimate blast radius after a signature change

Track:

- correctness of the downstream edit
- number of tool calls
- total tokens used
- unnecessary file reads
- false positives and false negatives

### Expected Benefits

- Measures what actually matters: whether the context helps an agent do the
  right work.
- Prevents local optimizations that make outputs smaller but less useful.
- Creates a durable benchmark for future algorithm changes.
- Makes it easier to justify more sophisticated semantic work with evidence.

### Cost / Tradeoffs

- Requires more setup than unit-style fixture tests.
- End-to-end scenarios can be noisier and need careful design to stay stable.

## 7. Relevance Explanations

### Idea

For ranked or filtered outputs, include compact reasons for inclusion.

Examples:

- `imported`
- `signature_ref`
- `field_ref`
- `implements_ref`
- `fallback`
- `same_module`

### Expected Benefits

- Makes ranking easier to debug and trust.
- Helps LLMs decide which result to inspect first.
- Gives maintainers a way to see why a bad result appeared without re-reading
  the full implementation.
- Useful for future evaluation metrics, because precision can be inspected by
  reason category.

### Cost / Tradeoffs

- More schema surface and some extra tokens.
- The reason taxonomy needs to stay small or it becomes noisy.

## 8. Incremental Symbol Graph / Caching

### Idea

Build a reusable project-level symbol graph that can be updated incrementally
rather than recomputed independently for each tool call.

### Expected Benefits

- Improves latency for multi-step workflows such as `find_usages` ->
  `affected_by_diff` -> `view_code`.
- Makes richer semantic analysis more practical without blowing up cost.
- Encourages consistency across tools because they share one underlying symbol
  model instead of each rebuilding partial views.
- Could enable future tools like workspace-level related symbol search and
  project-wide focused slices.

### Cost / Tradeoffs

- More architectural complexity.
- Cache invalidation and filesystem change tracking need careful handling.

## 9. Better Dependency Modes For `view_code`

### Idea

Expose dependency inclusion as an explicit policy instead of one mostly hidden
heuristic.

Suggested modes:

- `none`
- `strict`
- `related`
- `broad`

### Expected Benefits

- Lets users and agents ask for the right amount of context deliberately.
- Makes focused reads safer by default while still supporting exploratory
  dependency expansion when needed.
- Gives tests a clearer contract, because output differences become intentional
  mode differences rather than emergent heuristics.

### Cost / Tradeoffs

- Another user-facing control to document.
- Needs strong defaults so most users do not have to think about it.

## 10. MCP-Only Codebase Navigation

### Idea

Design the tool surface so an LLM can understand and navigate most repositories
through MCP responses alone, using raw file reads only as an exception path.

That means the MCP should answer questions like:

- what matters in this directory?
- what owns this line?
- what are the important types and symbols here?
- what should I inspect next?
- what code slice is sufficient for this task?

### Expected Benefits

- Strongly reduces token waste from large file dumps.
- Gives LLMs a consistent navigation workflow instead of ad hoc exploration.
- Makes the project more valuable as infrastructure, not just as a collection of
  helper tools.
- Encourages tool outputs to be shaped around actual agent decisions rather than
  around internal implementation convenience.

### Cost / Tradeoffs

- Raises the bar for the tool surface: gaps become more visible because agents
  depend on it end to end.
- Some tasks will still require raw code, so fallbacks need to stay available
  and clearly defined.

## 11. Symbol-First Diff And Change Slicing

### Idea

Lean harder into structural and semantic change views so the default workflow is
not "read a huge git diff", but "inspect the minimum relevant change slice".

Examples:

- changed symbols only
- impacted call sites only
- signature deltas only
- ownership and blast-radius summaries
- targeted before/after snippets for one symbol

### Expected Benefits

- Saves a large number of tokens during review and refactoring tasks.
- Better matches how coding LLMs reason: they usually need "what changed and why
  does it matter", not every edited line in the repository.
- Reduces noise from formatting churn, generated code, and unrelated hunks.
- Makes `parse_diff` and `affected_by_diff` more central to the product vision.

### Cost / Tradeoffs

- Harder than displaying raw diffs, because the slicing must be trustworthy.
- Some low-level review tasks still need line-level detail, so this should be a
  first stop, not the only possible view.

## 12. Next-Step Recommendation Tooling

### Idea

Add lightweight guidance about the smallest next read or next tool call that is
likely to resolve the current uncertainty.

Examples:

- "inspect symbol X next"
- "read focused slice from file Y"
- "resolve dependency type Z before editing"
- "review affected usage rows before renaming"

### Expected Benefits

- Cuts down exploratory thrashing.
- Helps agents stay on an MCP-first path instead of falling back to big raw
  reads too early.
- Can lower total tool-call count and total token cost across multi-step tasks.

### Cost / Tradeoffs

- Recommendation quality must be good enough to be trusted.
- Adds another layer of ranking logic that must stay deterministic.

## How To Use This File

Use this file for:

- strategic ideas
- optional enhancements
- future tool concepts
- evaluation ideas

Use [PLAN.md](./PLAN.md) for:

- committed remediation work
- milestones
- acceptance criteria
- execution order
