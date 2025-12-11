# Agent Guidelines for treesitter-mcp

## Build & Test Commands
- **Build**: `cargo build` (debug) or `cargo build --release` (optimized)
- **Test all**: `cargo test`
- **Single test**: `cargo test test_name` (e.g., `cargo test test_code_map_single_file`)
- **Test with output**: `cargo test -- --nocapture`
- **Lint**: `cargo clippy -- -D warnings` (must pass with zero warnings)
- **Format check**: `cargo fmt --check`
- **Auto-format**: `cargo fmt`

## Code Style & Conventions
- **TDD Approach**: Follow RED/GREEN/BLUE pattern - write tests first, implement to pass, refactor
- **Testing**: Write always test for business logic and requirements and if these are missing ask for them
- **Error Handling**: Use `eyre::Result<T>` for all fallible operations, propagate with `?` operator
- **Imports**: Group by category with blank lines: std → external crates → internal modules (e.g., `crate::mcp`, `crate::parser`)
- **Naming**: snake_case for functions/variables, PascalCase for types, SCREAMING_SNAKE_CASE for constants
- **Logging**: Use `log::info!`, `log::debug!`, `log::error!` macros with descriptive messages
- **Documentation**: Use `///` for public items and `//!` for module-level docs with clear purpose statements
- **Serde**: Use `#[serde(skip_serializing_if = "...")]` to omit empty/optional fields in JSON output
- **JSON-RPC**: MCP protocol uses JSON-RPC 2.0 over stdio (stdin/stdout), keep messages compact so as little tokens are used as possible(remove whitespace)
- **Tree-sitter**: Node types are language-specific, use S-expression queries, handle malformed code gracefully

## treesitter-mcp Tools
Use treesitter-mcp to understand code structure before making changes:
- **Exploring codebase?** → `code_map` on the directory
- **Before editing a file?** → `parse_file` to understand it fully
- **Refactoring or renaming?** → `find_usages` to check impact
- **After making changes?** → `parse_diff` to verify what changed at symbol level
- **Before running tests?** → `affected_by_diff` to see what might break
- **Got a line number?** → `get_context` to understand scope
- **Need quick file overview?** → `file_shape` for skeleton only

Prefer treesitter-mcp over grep for structural queries (finding functions, classes, usages).
