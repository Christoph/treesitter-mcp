//! Get Node At Position Tool (STUB - NOT IMPLEMENTED)
//!
//! This tool will get the AST node at a specific position with ancestors.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use serde_json::Value;
use std::io;

pub fn execute(_arguments: &Value) -> Result<CallToolResult, io::Error> {
    // STUB: Not implemented yet
    // This will be implemented in the GREEN phase
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "get_node_at_position tool not implemented yet",
    ))
}
