//! MCP Tool definitions and implementations
//!
//! This module defines all the tools provided by the treesitter-mcp server
//! using the rust-mcp-sdk macros and conventions.

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::tool_box;

use crate::analysis::{code_map, diff, find_usages, query_pattern, symbol_at_line, view_code};

// Helper function for serde default
fn default_full() -> String {
    "full".to_string()
}

fn default_one() -> Option<u32> {
    Some(1)
}

/// View a source file with flexible detail levels and automatic type inclusion
#[mcp_tool(
    name = "view_code",
    description = "View file with flexible detail levels. Returns code/signatures with automatic project type inclusion to prevent hallucinations. USE WHEN: ✅ Need to view/edit a file ✅ Want type definitions from dependencies ✅ Need full code or just signatures. DETAIL LEVELS: 'signatures' (skeleton only), 'full' (complete code). FOCUS: Set focus_symbol to get full code for one symbol + signatures for rest. AUTO-INCLUDES: All struct/class/interface definitions from project dependencies (not external libs). TOKEN COST: MEDIUM-HIGH. WORKFLOW: code_map → view_code"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ViewCode {
    /// Path to the source file
    pub file_path: String,

    /// Detail level: "signatures" or "full" (default: "full")
    /// - "signatures": Function/class signatures only (no bodies)
    /// - "full": Complete implementation code
    #[serde(default = "default_full")]
    pub detail: String,

    /// Optional: Focus on ONE symbol, show full code only for it
    /// When set, returns full code for this symbol + signatures for rest
    #[serde(default)]
    pub focus_symbol: Option<String>,
}

/// Generate a high-level code map of a directory with token budget awareness and detail levels
#[mcp_tool(
    name = "code_map",
    description = "Generate hierarchical map of a DIRECTORY (not single file). Returns structure overview of multiple files with functions/classes/types. Detail levels: 'minimal' (names only), 'signatures' (DEFAULT, names + signatures), 'full' (includes code). USE WHEN: ✅ First time exploring unfamiliar codebase ✅ Finding where functionality lives across files ✅ Getting project structure overview ✅ Don't know which file to examine. DON'T USE: ❌ Know specific file → use view_code ❌ Need implementation details → use view_code after identifying files. TOKEN COST: MEDIUM (scales with project size). OPTIMIZATION: Start with detail='minimal' for large projects, use pattern to filter. WORKFLOW: code_map → view_code"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct CodeMap {
    /// Path to file or directory
    pub path: String,
    /// Maximum tokens for output (approximate, default: 2000)
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Detail level: "minimal", "signatures", or "full" (default: "signatures")
    #[serde(default)]
    pub detail: Option<String>,
    /// Glob pattern to filter files (e.g., "*.rs")
    #[serde(default)]
    pub pattern: Option<String>,
}

/// Find all usages of a symbol with context and usage type classification
#[mcp_tool(
    name = "find_usages",
    description = "Find ALL usages of a symbol (function, variable, class, type) across files. Semantic search, not text search. Returns file locations, code context, usage type (definition, call, type_reference, import, reference). USE WHEN: ✅ Refactoring: see all places that call a function ✅ Impact analysis: checking what breaks if you change signature ✅ Tracing data flow ✅ Before renaming/modifying shared code. DON'T USE: ❌ Need structural changes only → use parse_diff ❌ Want risk assessment → use affected_by_diff ❌ Symbol used >50 places → use affected_by_diff or set max_context_lines=50. TOKEN COST: MEDIUM-HIGH (scales with usage count × context_lines). OPTIMIZATION: Set max_context_lines=50 for frequent symbols, context_lines=1 for locations only. WORKFLOW: find_usages (before changes) → make changes → affected_by_diff (verify)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct FindUsages {
    /// Symbol name to search for
    pub symbol: String,
    /// File or directory path to search in
    pub path: String,
    /// Number of context lines around each usage (default: 3)
    #[serde(default)]
    pub context_lines: Option<u32>,
    /// Maximum total context lines across ALL usages (prevents token explosion)
    /// When set, limits the total number of context lines returned
    #[serde(default)]
    pub max_context_lines: Option<u32>,
}

/// Get symbol information at a specific line with signature and scope chain
#[mcp_tool(
    name = "symbol_at_line",
    description = "Get symbol (function/class/method) at specific line with signature and scope chain. Returns symbol name, signature, kind, and enclosing scopes from innermost to outermost. USE WHEN: ✅ Have line number from error/stack trace ✅ Need to know 'what function is this line in?' ✅ Want function signature at a location ✅ Understanding scope hierarchy. DON'T USE: ❌ Need full code → use view_code with focus_symbol ❌ Know symbol name already → use view_code directly. TOKEN COST: LOW. WORKFLOW: symbol_at_line (find symbol) → view_code (see code)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct SymbolAtLine {
    /// Path to the source file
    pub file_path: String,

    /// Line number (1-indexed)
    pub line: u32,

    /// Column number (1-indexed, default: 1)
    #[serde(default = "default_one")]
    pub column: Option<u32>,
}

/// Analyze structural changes in a file compared to a git revision
#[mcp_tool(
    name = "parse_diff",
    description = "Analyze structural changes vs git revision. Returns symbol-level diff (functions/classes added/removed/modified), not line-level. USE WHEN: ✅ Verifying what you changed at structural level ✅ Checking if changes are cosmetic (formatting) or substantive ✅ Understanding changes without re-reading entire file ✅ Generating change summaries. DON'T USE: ❌ Need to see what might break → use affected_by_diff ❌ Haven't made changes yet → use view_code ❌ Need line-by-line diff → use git diff. TOKEN COST: LOW-MEDIUM (much smaller than re-reading file). WORKFLOW: After changes: parse_diff (verify) → affected_by_diff (check impact)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseDiff {
    /// Path to the source file to analyze
    pub file_path: String,
    /// Git revision to compare against (default: "HEAD")
    /// Examples: "HEAD", "HEAD~1", "main", "abc123"
    #[serde(default)]
    pub compare_to: Option<String>,
}

/// Find usages that might be affected by changes in a file
#[mcp_tool(
    name = "affected_by_diff",
    description = "Find usages AFFECTED by your changes. Combines parse_diff + find_usages to show blast radius with risk levels (HIGH/MEDIUM/LOW) based on change type. USE WHEN: ✅ After modifying function signatures - what might break? ✅ Before running tests - anticipate failures ✅ During refactoring - understand impact radius ✅ Risk assessment for code changes. DON'T USE: ❌ Haven't made changes yet → use find_usages first ❌ Just want to see what changed → use parse_diff ❌ Changes are purely internal (no signature changes) → parse_diff is enough. TOKEN COST: MEDIUM-HIGH (combines parse_diff + find_usages). OPTIMIZATION: Use scope parameter to limit search area. WORKFLOW: parse_diff (see changes) → affected_by_diff (assess impact) → fix issues"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct AffectedByDiff {
    /// Path to the changed source file
    pub file_path: String,
    /// Git revision to compare against (default: "HEAD")
    #[serde(default)]
    pub compare_to: Option<String>,
    /// Directory to search for affected usages (default: project root)
    #[serde(default)]
    pub scope: Option<String>,
}

/// Execute a custom tree-sitter query pattern on a source file with code context
#[mcp_tool(
    name = "query_pattern",
    description = "Execute custom tree-sitter S-expression query for advanced AST pattern matching. Returns matches with code context for complex structural patterns. USE WHEN: ✅ Finding all instances of specific syntax pattern (e.g., all if statements) ✅ Complex structural queries (e.g., all async functions with try-catch) ✅ Language-specific patterns find_usages can't handle ✅ You know tree-sitter query syntax. DON'T USE: ❌ Finding function/variable usages → use find_usages (simpler, cross-language) ❌ Don't know tree-sitter syntax → use find_usages or view_code ❌ Simple symbol search → use find_usages. TOKEN COST: MEDIUM (depends on matches). COMPLEXITY: HIGH - requires tree-sitter query knowledge. RECOMMENDATION: Prefer find_usages for 90% of use cases."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct QueryPattern {
    /// Path to the source file
    pub file_path: String,
    /// Tree-sitter query pattern in S-expression format
    pub query: String,
    /// Number of context lines around each match (default: 2)
    #[serde(default)]
    pub context_lines: Option<u32>,
}

// Implement tool execution logic for each tool
impl ViewCode {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "detail": self.detail,
            "focus_symbol": self.focus_symbol
        });

        view_code::execute(&args).map_err(CallToolError::new)
    }
}

impl CodeMap {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "path": self.path,
            "max_tokens": self.max_tokens.unwrap_or(2000),
            "detail": self.detail,
            "pattern": self.pattern
        });

        code_map::execute(&args).map_err(CallToolError::new)
    }
}

impl FindUsages {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "symbol": self.symbol,
            "path": self.path,
            "context_lines": self.context_lines,
            "max_context_lines": self.max_context_lines
        });

        find_usages::execute(&args).map_err(CallToolError::new)
    }
}

impl SymbolAtLine {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "line": self.line,
            "column": self.column
        });

        symbol_at_line::execute(&args).map_err(CallToolError::new)
    }
}

impl ParseDiff {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "compare_to": self.compare_to
        });

        diff::execute_parse_diff(&args).map_err(CallToolError::new)
    }
}

impl AffectedByDiff {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "compare_to": self.compare_to,
            "scope": self.scope
        });

        diff::execute_affected_by_diff(&args).map_err(CallToolError::new)
    }
}

impl QueryPattern {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "query": self.query,
            "context_lines": self.context_lines
        });

        query_pattern::execute(&args).map_err(CallToolError::new)
    }
}

// Generate an enum with all tools
tool_box!(
    TreesitterTools,
    [
        ViewCode,
        CodeMap,
        FindUsages,
        SymbolAtLine,
        ParseDiff,
        AffectedByDiff,
        QueryPattern
    ]
);
