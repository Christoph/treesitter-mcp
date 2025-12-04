//! Get Context Tool (STUB - NOT IMPLEMENTED)
//!
//! This tool will get the enclosing context at a specific position.

use crate::mcp_types::{CallToolResult, CallToolResultExt};
use serde_json::Value;
use std::io;

pub fn execute(_arguments: &Value) -> Result<CallToolResult, io::Error> {
    // STUB: Not implemented yet
    // This will be implemented in the GREEN phase
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "get_context tool not implemented yet",
    ))
}
