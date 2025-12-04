//! MCP Tool definitions and implementations
//!
//! This module defines all the tools provided by the treesitter-mcp server
//! using the rust-mcp-sdk macros and conventions.

use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use rust_mcp_sdk::schema::{schema_utils::CallToolError, CallToolResult};
use rust_mcp_sdk::tool_box;

use crate::analysis::{code_map, file_shape, find_usages, parse_file, query_pattern};

/// Parse a source file using tree-sitter and return the AST as an S-expression
#[mcp_tool(
    name = "parse_file",
    description = "Parse a source file using tree-sitter and return the AST as an S-expression. Use this to inspect the raw Abstract Syntax Tree structure of a file, revealing the exact syntactic hierarchy as seen by Tree-sitter. Essential for debugging parsing logic, understanding node relationships, or designing precise structural queries."
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

/// Generate a high-level code map of a directory with token budget awareness
#[mcp_tool(
    name = "code_map",
    description = "Generate a high-level code map of a directory with token budget awareness. Provides a hierarchical overview of the codebase structure, useful for understanding project organization and navigating large codebases efficiently."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct CodeMapTool {
    /// Path to file or directory
    pub path: String,
    /// Maximum tokens for output (approximate, default: 2000)
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Find all usages of a symbol (function, struct, class) in a file or directory
#[mcp_tool(
    name = "find_usages",
    description = "Find all usages of a symbol (function, struct, class) in a file or directory. Searches for references to a specific symbol throughout the codebase, helping with refactoring, impact analysis, and understanding code dependencies."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct FindUsagesTool {
    /// Symbol name to search for
    pub symbol: String,
    /// File or directory path to search in
    pub path: String,
}

/// Execute a custom tree-sitter query pattern on a source file
#[mcp_tool(
    name = "query_pattern",
    description = "Execute a custom tree-sitter query pattern on a source file. Allows advanced code analysis using tree-sitter's query language in S-expression format. Useful for custom code searches, linting, or extracting specific code patterns."
)]
#[derive(Debug, ::serde::Deserialize, ::serde::Serialize, JsonSchema)]
pub struct QueryPatternTool {
    /// Path to the source file
    pub file_path: String,
    /// Tree-sitter query pattern in S-expression format
    pub query: String,
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
            "max_tokens": self.max_tokens.unwrap_or(2000)
        });

        code_map::execute(&args).map_err(CallToolError::new)
    }
}

impl FindUsagesTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "symbol": self.symbol,
            "path": self.path
        });

        find_usages::execute(&args).map_err(CallToolError::new)
    }
}

impl QueryPatternTool {
    pub fn call_tool(&self) -> Result<CallToolResult, CallToolError> {
        let args = serde_json::json!({
            "file_path": self.file_path,
            "query": self.query
        });

        query_pattern::execute(&args).map_err(CallToolError::new)
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
        QueryPatternTool
    ]
);
