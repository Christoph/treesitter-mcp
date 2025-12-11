//! MCP Tool definitions and implementations
//!
//! This module defines all the tools provided by the treesitter-mcp server
//! using the rust-mcp-sdk macros and conventions.

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::tool_box;

use crate::analysis::{
    code_map, diff, file_shape, find_usages, get_context, get_node_at_position, parse_file,
    query_pattern, read_focused_code,
};

// Helper function for serde default
fn default_true() -> bool {
    true
}

/// Parse a source file and return its structure (functions, classes, imports) with signatures and docs
#[mcp_tool(
    name = "parse_file",
    description = "Parse single file with FULL implementation details. Returns complete code for all functions/classes with names, signatures, line ranges, and doc comments. USE WHEN: ✅ Understanding implementation before editing ✅ File <500 lines needing complete context ✅ Modifying multiple functions in same file. DON'T USE: ❌ Only need signatures → use file_shape (10x cheaper) ❌ Only editing one function → use read_focused_code (3x cheaper) ❌ File >500 lines → use file_shape first. TOKEN COST: HIGH. OPTIMIZATION: Set include_code=false for 60-80% reduction."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseFileTool {
    /// Path to the source file to parse
    pub file_path: String,
    /// Include full code blocks for functions/classes (default: true)
    /// When false, returns only signatures, docs, and line ranges (60-80% token reduction)
    #[serde(default = "default_true")]
    pub include_code: bool,
}

/// Read a file with focused code view: FULL code for the target symbol,
/// signatures-only for everything else. Perfect for editing a specific function
/// while maintaining context of the surrounding code.
#[mcp_tool(
    name = "read_focused_code",
    description = "Read file with FULL code for ONE symbol, signatures-only for everything else. Returns complete implementation of target function/class plus signatures of surrounding code. USE WHEN: ✅ Know exactly which function to edit ✅ Need surrounding context for dependencies ✅ File is large but only care about one function ✅ Want to minimize tokens while maintaining context. DON'T USE: ❌ Need multiple functions → use parse_file ❌ Don't know which function → use file_shape first. TOKEN COST: MEDIUM (~30% of parse_file). OPTIMIZATION: Keep context_radius=0 unless need adjacent functions. WORKFLOW: file_shape → read_focused_code"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ReadFocusedCodeTool {
    /// Path to the source file
    pub file_path: String,

    /// Symbol name to show full code for (function, class, or struct name)
    pub focus_symbol: String,

    /// Include full code for N symbols before/after the focused symbol (default: 0)
    #[serde(default)]
    pub context_radius: Option<u32>,
}

/// Extract the structure of a file (functions, classes, imports) without implementation details
#[mcp_tool(
    name = "file_shape",
    description = "Extract file structure WITHOUT implementation code. Returns skeleton: function/class signatures, imports, dependencies only (NO function bodies). For HTML/CSS: returns IDs, classes, theme variables. USE WHEN: ✅ Quick overview of file's API/interface ✅ Deciding which function to focus on before read_focused_code ✅ Mapping dependencies (use include_deps=true) ✅ File >500 lines needing orientation. DON'T USE: ❌ Need implementation logic → use parse_file or read_focused_code ❌ Exploring multiple files → use code_map. TOKEN COST: LOW (10-20% of parse_file). OPTIMIZATION: Use this FIRST, then drill down. WORKFLOW: file_shape → read_focused_code → parse_file (if needed)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct FileShapeTool {
    /// Path to the source file
    pub file_path: String,
    /// Include project dependencies as nested file shapes (default: false)
    #[serde(default)]
    pub include_deps: bool,
    /// For Askama/Jinja2 templates (.html in templates/ dir): merge extends/includes into single output. Returns error if used on non-template files. (default: false)
    #[serde(default)]
    pub merge_templates: bool,
}

/// Generate a high-level code map of a directory with token budget awareness and detail levels
#[mcp_tool(
    name = "code_map",
    description = "Generate hierarchical map of a DIRECTORY (not single file). Returns structure overview of multiple files with functions/classes/types. Detail levels: 'minimal' (names only), 'signatures' (DEFAULT, names + signatures), 'full' (includes code). USE WHEN: ✅ First time exploring unfamiliar codebase ✅ Finding where functionality lives across files ✅ Getting project structure overview ✅ Don't know which file to examine. DON'T USE: ❌ Know specific file → use file_shape or parse_file ❌ Need implementation details → use parse_file after identifying files. TOKEN COST: MEDIUM (scales with project size). OPTIMIZATION: Start with detail='minimal' for large projects, use pattern to filter. WORKFLOW: code_map → file_shape → parse_file/read_focused_code"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct CodeMapTool {
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
pub struct FindUsagesTool {
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

/// Execute a custom tree-sitter query pattern on a source file with code context
#[mcp_tool(
    name = "query_pattern",
    description = "Execute custom tree-sitter S-expression query for advanced AST pattern matching. Returns matches with code context for complex structural patterns. USE WHEN: ✅ Finding all instances of specific syntax pattern (e.g., all if statements) ✅ Complex structural queries (e.g., all async functions with try-catch) ✅ Language-specific patterns find_usages can't handle ✅ You know tree-sitter query syntax. DON'T USE: ❌ Finding function/variable usages → use find_usages (simpler, cross-language) ❌ Don't know tree-sitter syntax → use find_usages or parse_file ❌ Simple symbol search → use find_usages. TOKEN COST: MEDIUM (depends on matches). COMPLEXITY: HIGH - requires tree-sitter query knowledge. RECOMMENDATION: Prefer find_usages for 90% of use cases."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct QueryPatternTool {
    /// Path to the source file
    pub file_path: String,
    /// Tree-sitter query pattern in S-expression format
    pub query: String,
    /// Number of context lines around each match (default: 2)
    #[serde(default)]
    pub context_lines: Option<u32>,
}

/// Get the enclosing context (function, class, module) at a specific position
#[mcp_tool(
    name = "get_context",
    description = "Get enclosing scope hierarchy at specific file:line:column position. Returns nested contexts from innermost to outermost (e.g., 'inside function X, inside class Y, inside module Z'). USE WHEN: ✅ Have line number from error/stack trace/user reference ✅ Need to know 'what function is this line in?' ✅ Understanding scope hierarchy for debugging ✅ Navigating to specific location in code. DON'T USE: ❌ Need actual code → use read_focused_code after getting function name ❌ Need detailed AST info → use get_node_at_position ❌ Know function name already → use read_focused_code directly. TOKEN COST: LOW (just scope chain). WORKFLOW: get_context (find function) → read_focused_code (see implementation)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct GetContextTool {
    /// Path to the source file
    pub file_path: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed, default: 1)
    #[serde(default)]
    pub column: Option<u32>,
}

/// Get the AST node at a specific position with ancestor chain
#[mcp_tool(
    name = "get_node_at_position",
    description = "Get precise AST node at file:line:column position with parent chain. Returns node type, text, range, N ancestor nodes. USE WHEN: ✅ Need exact syntactic information at cursor position ✅ Syntax-aware edits (e.g., wrap this expression in function call) ✅ Understanding what token/expression is at location ✅ Debugging parse issues or AST structure. DON'T USE: ❌ Just need function name → use get_context (simpler) ❌ Need full function code → use read_focused_code ❌ Not doing syntax-aware operations → use get_context. TOKEN COST: LOW (just node info). COMPLEXITY: MEDIUM - requires understanding AST concepts. USE CASE: Advanced/syntax-aware operations only."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct GetNodeAtPositionTool {
    /// Path to the source file
    pub file_path: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Number of ancestor levels to return (default: 3)
    #[serde(default)]
    pub ancestor_levels: Option<u32>,
}

/// Analyze structural changes in a file compared to a git revision
#[mcp_tool(
    name = "parse_diff",
    description = "Analyze structural changes vs git revision. Returns symbol-level diff (functions/classes added/removed/modified), not line-level. USE WHEN: ✅ Verifying what you changed at structural level ✅ Checking if changes are cosmetic (formatting) or substantive ✅ Understanding changes without re-reading entire file ✅ Generating change summaries. DON'T USE: ❌ Need to see what might break → use affected_by_diff ❌ Haven't made changes yet → use parse_file ❌ Need line-by-line diff → use git diff. TOKEN COST: LOW-MEDIUM (much smaller than re-reading file). WORKFLOW: After changes: parse_diff (verify) → affected_by_diff (check impact)"
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseDiffTool {
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
pub struct AffectedByDiffTool {
    /// Path to the changed source file
    pub file_path: String,
    /// Git revision to compare against (default: "HEAD")
    #[serde(default)]
    pub compare_to: Option<String>,
    /// Directory to search for affected usages (default: project root)
    #[serde(default)]
    pub scope: Option<String>,
}

// Implement tool execution logic for each tool
impl ParseFileTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "include_code": self.include_code
        });

        parse_file::execute(&args).map_err(CallToolError::new)
    }
}

impl ReadFocusedCodeTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "focus_symbol": self.focus_symbol,
            "context_radius": self.context_radius.unwrap_or(0)
        });

        read_focused_code::execute(&args).map_err(CallToolError::new)
    }
}

impl FileShapeTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "include_deps": self.include_deps
        });

        file_shape::execute(&args).map_err(CallToolError::new)
    }
}

impl CodeMapTool {
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

impl FindUsagesTool {
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

impl QueryPatternTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "query": self.query,
            "context_lines": self.context_lines
        });

        query_pattern::execute(&args).map_err(CallToolError::new)
    }
}

impl GetContextTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "line": self.line,
            "column": self.column
        });

        get_context::execute(&args).map_err(CallToolError::new)
    }
}

impl GetNodeAtPositionTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "line": self.line,
            "column": self.column,
            "ancestor_levels": self.ancestor_levels
        });

        get_node_at_position::execute(&args).map_err(CallToolError::new)
    }
}

impl ParseDiffTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "compare_to": self.compare_to
        });

        diff::execute_parse_diff(&args).map_err(CallToolError::new)
    }
}

impl AffectedByDiffTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "compare_to": self.compare_to,
            "scope": self.scope
        });

        diff::execute_affected_by_diff(&args).map_err(CallToolError::new)
    }
}

// Generate an enum with all tools
tool_box!(
    TreesitterTools,
    [
        ParseFileTool,
        ReadFocusedCodeTool,
        FileShapeTool,
        CodeMapTool,
        FindUsagesTool,
        QueryPatternTool,
        GetContextTool,
        GetNodeAtPositionTool,
        ParseDiffTool,
        AffectedByDiffTool
    ]
);
