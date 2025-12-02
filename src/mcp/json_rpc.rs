use serde::{Deserialize, Serialize};
use serde_json::Value;
use eyre::Result;

/// JSON-RPC 2.0 message (request or notification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub jsonrpc: String,
    /// Request ID - present for requests, absent for notifications
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    /// Method name to invoke
    pub method: String,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    /// Must match the request ID
    pub id: Value,
    /// Success result - mutually exclusive with error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error object - mutually exclusive with result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorObject>,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorObject {
    /// Error code (standard codes are negative)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Parse a JSON-RPC message from a JSON string
///
/// # Errors
/// Returns an error if the JSON is invalid or doesn't match the Message schema
pub fn parse_message(json_str: &str) -> Result<Message> {
    let msg = serde_json::from_str(json_str)?;
    Ok(msg)
}

/// Serialize a response to compact JSON (no whitespace)
///
/// # Panics
/// Panics if the response cannot be serialized (should never happen with valid Response)
pub fn serialize_response(response: &Response) -> String {
    serde_json::to_string(response)
        .expect("Response should always be serializable")
}

/// Create a JSON-RPC error response
///
/// # Arguments
/// * `id` - The request ID to respond to
/// * `code` - JSON-RPC error code (e.g., -32601 for method not found)
/// * `message` - Human-readable error description
pub fn create_error_response(id: Value, code: i32, message: &str) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(ErrorObject {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}

/// Standard JSON-RPC 2.0 error codes
#[allow(dead_code)]
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // MCP-specific error codes
    pub const SERVER_NOT_INITIALIZED: i32 = -32002;
    pub const RESOURCE_NOT_FOUND: i32 = -32002;
}
