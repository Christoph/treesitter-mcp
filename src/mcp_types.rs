//! MCP types re-export
//!
//! This module re-exports MCP types from rust-mcp-sdk for use in analysis modules.

pub use rust_mcp_sdk::schema::{CallToolResult, TextContent};

// Helper extension trait for CallToolResult
pub trait CallToolResultExt {
    fn success(text: String) -> Self;
}

impl CallToolResultExt for CallToolResult {
    fn success(text: String) -> Self {
        CallToolResult::text_content(vec![TextContent::from(text)])
    }
}
