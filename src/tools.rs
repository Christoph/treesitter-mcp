//! MCP Tool definitions and implementations
//!
//! This module defines all the tools provided by the treesitter-mcp server
//! using the rust-mcp-sdk macros and conventions.

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::tool_box;

use crate::analysis::{
    code_map, file_shape, find_usages, get_context, get_node_at_position, parse_file, query_pattern,
};

/// Parse a source file and return its structure (functions, classes, imports) with signatures and docs
#[mcp_tool(
    name = "parse_file",
    description = "Parse a single source file and return complete structure with FULL implementation details. Returns: function/class names, signatures, line ranges, doc comments, and complete code blocks. USE THIS WHEN: you need to read, understand, or modify a specific file's code. This is the primary tool for examining file contents before editing."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseFileTool {
    /// Path to the source file to parse
    pub file_path: String,
}

/// Extract the structure of a file (functions, classes, imports) without implementation details
#[mcp_tool(
    name = "file_shape",
    description = "Extract file structure as a skeleton WITHOUT implementation code. Returns: function/class signatures, imports, and dependencies only - no function bodies. For HTML/CSS: returns IDs, custom classes, theme variables. For templates in templates/ dir: use merge_templates=true to get merged content with extends/includes resolved. USE THIS WHEN: you need a quick overview of a file's API/interface, want to understand imports and exports, or are mapping dependencies. Faster and smaller output than parse_file."
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
    description = "Generate a hierarchical map of a DIRECTORY (not single file). Scans multiple files and returns structure overview. Detail levels: 'minimal' (names only), 'signatures' (names + signatures, DEFAULT), 'full' (everything). USE THIS WHEN: exploring unfamiliar codebases, finding where code lives, or getting project overview. Respects token budget to avoid context overflow."
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
    description = "Find ALL usages of a symbol (function, variable, class, type) across files. Returns: file locations, surrounding code context, and usage type (definition, call, type_reference, import, reference). USE THIS WHEN: refactoring, checking impact of changes, finding where something is called/used, or tracing data flow. Essential before renaming or modifying shared code."
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
}

/// Execute a custom tree-sitter query pattern on a source file with code context
#[mcp_tool(
    name = "query_pattern",
    description = "Execute a custom tree-sitter S-expression query for advanced AST matching. USE THIS WHEN: you need precise pattern matching that other tools don't cover, such as finding all 'if' statements, all async functions, or complex structural patterns. Requires knowledge of tree-sitter query syntax. For most tasks, prefer find_usages or parse_file instead."
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
    description = "Get the enclosing scope hierarchy at a specific file:line:column position. Returns: nested contexts from innermost to outermost (e.g., 'this line is inside function X, which is inside class Y, which is in module Z'). USE THIS WHEN: you have a line number from an error, stack trace, or user reference and need to understand what scope/function it belongs to."
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
    description = "Get the precise AST node at a file:line:column position with its parent chain. Returns: node type, text, range, and N ancestor nodes. USE THIS WHEN: you need exact syntactic information at a cursor position - for syntax-aware edits, understanding what token/expression is at a location, or debugging parse issues. More granular than get_context."
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

// Implement tool execution logic for each tool
impl ParseFileTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path
        });

        parse_file::execute(&args).map_err(CallToolError::new)
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
            "context_lines": self.context_lines
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

// Generate an enum with all tools
tool_box!(
    TreesitterTools,
    [
        ParseFileTool,
        FileShapeTool,
        CodeMapTool,
        FindUsagesTool,
        QueryPatternTool,
        GetContextTool,
        GetNodeAtPositionTool
    ]
);
