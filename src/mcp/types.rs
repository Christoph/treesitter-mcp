use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Initialization Types
// ============================================================================

/// Client information sent during MCP initialization
///
/// Identifies the client application connecting to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client application name (e.g., "claude-desktop")
    pub name: String,
    /// Client version (e.g., "1.0.0")
    pub version: String,
}

/// Server information returned during MCP initialization
///
/// Identifies this MCP server implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name (e.g., "treesitter-mcp")
    pub name: String,
    /// Server version (e.g., "0.1.0")
    pub version: String,
}

/// Client capabilities declaration
///
/// Tells the server what features the client supports
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Tools capability marker
///
/// Indicates the server exposes MCP tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {}

/// Server capabilities exposed during initialization
///
/// Tells the client what features this server supports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Server provides tools (code analysis capabilities)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
    /// Server provides resources (not implemented yet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
    /// Server provides prompts (not implemented yet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
}

/// Parameters for the `initialize` request
///
/// Sent by the client to negotiate protocol version and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// MCP protocol version (must be "2025-11-25")
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client identification
    pub client_info: ClientInfo,
}

/// Result of the `initialize` request
///
/// Server responds with its protocol version, capabilities, and info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// MCP protocol version this server implements
    pub protocol_version: String,
    /// Server capabilities (tools, resources, prompts)
    pub capabilities: ServerCapabilities,
    /// Server identification
    pub server_info: ServerInfo,
}

// ============================================================================
// Tool Types
// ============================================================================

/// Tool definition with JSON schema for parameters
///
/// Describes a tool that can be invoked by the client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    /// Unique tool name (e.g., "parse_file")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for tool parameters
    pub input_schema: Value,
}

/// Parameters for the `tools/call` request
///
/// Specifies which tool to call and with what arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallParams {
    /// Tool name to invoke
    pub name: String,
    /// Tool-specific arguments (validated against inputSchema)
    pub arguments: Value,
}

/// Result of the `tools/list` request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    /// Array of available tools
    pub tools: Vec<ToolDefinition>,
}

// ============================================================================
// Content Types
// ============================================================================

/// Text content in a tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    /// Content type discriminator (always "text")
    #[serde(rename = "type")]
    pub type_: String,
    /// The actual text content
    pub text: String,
}

impl TextContent {
    /// Create a new text content with the given text
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            type_: "text".to_string(),
            text: text.into(),
        }
    }
}

/// Content union for tool results
///
/// MCP supports multiple content types (text, images, resources)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// Plain text content
    Text(TextContent),
    // Future: Image, Resource, etc.
}

/// Result of a tool invocation
///
/// Contains the tool output and error status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Array of content items (text, images, etc.)
    pub content: Vec<Content>,
    /// Whether this result represents an error
    #[serde(default)]
    pub is_error: bool,
}

impl CallToolResult {
    /// Create a successful result with text content
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(TextContent::new(text))],
            is_error: false,
        }
    }

    /// Create an error result with text content
    pub fn error(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text(TextContent::new(text))],
            is_error: true,
        }
    }
}

// ============================================================================
// Constants
// ============================================================================

/// List of all supported protocol versions (latest first)
pub const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2024-11-05"];
