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
    description = "Parse a source file and return its structure (functions, classes, imports) with full signatures, documentation, and code. Returns structured JSON with function/class names, signatures, line ranges, doc comments, and complete code blocks. Use this to understand file structure with implementation details."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct ParseFileTool {
    /// Path to the source file to parse
    pub file_path: String,
}

/// Extract the structure of a file (functions, classes, imports) without implementation details
#[mcp_tool(
    name = "file_shape",
    description = "Extract the structure of a file (functions, classes, imports) without implementation details. Use this to 'skeletonize' code files and quickly understand the interface and dependencies without reading the full implementation. Primarily used for generating file summaries, mapping dependency graphs, and understanding the high-level organization of code."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct FileShapeTool {
    /// Path to the source file
    pub file_path: String,
    /// Include project dependencies as nested file shapes (default: false)
    #[serde(default)]
    pub include_deps: bool,
}

/// Generate a high-level code map of a directory with token budget awareness and detail levels
#[mcp_tool(
    name = "code_map",
    description = "Generate a high-level code map of a directory with configurable detail levels. Supports three modes: 'minimal' (names only), 'signatures' (names + signatures), 'full' (names + signatures + docs + code). Provides a hierarchical overview of the codebase structure with token budget awareness."
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
    description = "Find all usages of a symbol (function, struct, class) with code context and usage type classification. Returns each usage with surrounding code lines, usage type (definition, call, type_reference, import, reference), and AST node information. Essential for refactoring and impact analysis."
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
    description = "Execute a custom tree-sitter query pattern on a source file with optional code context. Allows advanced code analysis using tree-sitter's query language in S-expression format. Returns matches with surrounding code lines and parent node information."
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
    description = "Get the enclosing context (function, class, module) at a specific position in a file. Returns a hierarchical list of contexts from innermost to outermost, with full signatures, code, and range information. Essential for understanding code structure at a cursor position."
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
    description = "Get the AST node at a specific position with its ancestor chain. Returns the smallest node at the position plus up to N ancestor nodes, with type, text, range, and name information. Useful for precise AST navigation and code analysis at specific positions."
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
