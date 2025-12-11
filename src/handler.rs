//! MCP Server Handler Implementation
//!
//! This module implements the ServerHandler trait to handle MCP protocol messages
//! and route tool calls to the appropriate implementations.

use async_trait::async_trait;
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequest, CallToolResult, ListToolsRequest,
    ListToolsResult, RpcError,
};
use rust_mcp_sdk::{mcp_server::ServerHandler, McpServer};
use std::sync::Arc;

use crate::tools::TreesitterTools;

/// Custom handler for tree-sitter MCP server
pub struct TreesitterServerHandler;

impl Default for TreesitterServerHandler {
    fn default() -> Self {
        Self
    }
}

impl TreesitterServerHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServerHandler for TreesitterServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: TreesitterTools::tools(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        log::info!("Calling tool: {}", request.tool_name());

        // Convert request params into the TreesitterTools enum
        let tool: TreesitterTools = TreesitterTools::try_from(request.params)?;

        // Match the tool variant and execute its corresponding logic
        match tool {
            TreesitterTools::ParseFile(t) => t.call_tool(),
            TreesitterTools::ReadFocusedCode(t) => t.call_tool(),
            TreesitterTools::FileShape(t) => t.call_tool(),
            TreesitterTools::CodeMap(t) => t.call_tool(),
            TreesitterTools::FindUsages(t) => t.call_tool(),
            TreesitterTools::QueryPattern(t) => t.call_tool(),
            TreesitterTools::GetContext(t) => t.call_tool(),
            TreesitterTools::GetNodeAtPosition(t) => t.call_tool(),
            TreesitterTools::ParseDiff(t) => t.call_tool(),
            TreesitterTools::AffectedByDiff(t) => t.call_tool(),
        }
    }
}
