use crate::mcp::json_rpc::{create_error_response, error_codes, Message, Response};
use crate::mcp::tool_registry::ToolRegistry;
use crate::mcp::types::{
    InitializeParams, InitializeResult, ListToolsResult, ServerCapabilities, ServerInfo,
    ToolCallParams, ToolsCapability, SUPPORTED_PROTOCOL_VERSIONS,
};
use eyre::{Result, WrapErr};
use serde_json::{json, Value};

/// MCP Server lifecycle state
///
/// The server follows the MCP initialization protocol:
/// 1. Uninitialized - Server created, waiting for initialize request
/// 2. Initializing - Initialize received, waiting for initialized notification
/// 3. Ready - Fully initialized and ready to handle all requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerState {
    /// Server created but not initialized
    Uninitialized,
    /// Initialize received but `notifications/initialized` not yet received
    Initializing,
    /// Server is ready to handle requests
    Ready,
}

/// MCP Server implementation
///
/// Handles the full MCP protocol lifecycle:
/// - Initialization handshake
/// - Request/notification routing
/// - State management
/// - Tool invocation (future)
pub struct McpServer {
    /// Current server state
    state: ServerState,
    /// Tool registry
    tool_registry: ToolRegistry,
}

impl McpServer {
    /// Create a new MCP server in uninitialized state
    pub fn new() -> Self {
        log::debug!("Creating new MCP server");

        let mut tool_registry = ToolRegistry::new();

        // Register all available tools
        tool_registry.register(crate::analysis::parse_file::tool_definition());
        tool_registry.register(crate::analysis::file_shape::tool_definition());
        tool_registry.register(crate::analysis::code_map::tool_definition());
        tool_registry.register(crate::analysis::find_usages::tool_definition());
        tool_registry.register(crate::analysis::query_pattern::tool_definition());

        Self {
            state: ServerState::Uninitialized,
            tool_registry,
        }
    }

    /// Check if the server is fully initialized and ready
    ///
    /// Returns true only after both `initialize` request and
    /// `notifications/initialized` notification have been received
    pub fn is_initialized(&self) -> bool {
        self.state == ServerState::Ready
    }

    /// Handle a JSON-RPC message and return a response
    ///
    /// This is the main entry point for message processing.
    ///
    /// # Arguments
    /// * `json_str` - JSON-RPC 2.0 message as a string
    ///
    /// # Returns
    /// - For requests: JSON-RPC response as a string
    /// - For notifications: Empty string (no response needed)
    ///
    /// # Errors
    /// Returns an error if:
    /// - The message is not valid JSON
    /// - The message doesn't conform to JSON-RPC 2.0
    /// - Internal processing fails
    pub fn handle_message(&mut self, json_str: &str) -> Result<String> {
        // Parse the JSON-RPC message
        let message: Message =
            serde_json::from_str(json_str).wrap_err("Failed to parse JSON-RPC message")?;

        match self.process_message(message)? {
            Some(response) => {
                let response_str =
                    serde_json::to_string(&response).wrap_err("Failed to serialize response")?;
                Ok(response_str)
            }
            None => Ok(String::new()),
        }
    }

    /// Process a parsed JSON-RPC message
    ///
    /// # Arguments
    /// * `message` - Parsed JSON-RPC message
    ///
    /// # Returns
    /// - `Ok(Some(Response))` for requests (which expect a response)
    /// - `Ok(None)` for notifications (which don't expect a response)
    /// - `Err(_)` for internal processing errors
    pub fn process_message(&mut self, message: Message) -> Result<Option<Response>> {
        log::debug!("Handling method: {}", message.method);

        // Notifications have no ID and expect no response
        if message.id.is_none() {
            self.handle_notification(&message)?;
            return Ok(None);
        }

        // Requests have an ID and expect a response
        let response = self.handle_request(&message)?;
        Ok(Some(response))
    }

    /// Handle a notification (no ID, no response expected)
    ///
    /// Notifications are used for:
    /// - `notifications/initialized` - Complete initialization
    /// - Other protocol notifications
    fn handle_notification(&mut self, message: &Message) -> Result<()> {
        match message.method.as_str() {
            "notifications/initialized" => {
                log::info!("Received initialized notification");
                if self.state == ServerState::Initializing {
                    self.state = ServerState::Ready;
                    log::info!("Server is now ready");
                }
                Ok(())
            }
            _ => {
                // Unknown notifications are silently ignored per JSON-RPC spec
                log::debug!("Ignoring unknown notification: {}", message.method);
                Ok(())
            }
        }
    }

    /// Handle a request (has ID, needs response)
    ///
    /// Validates server state and dispatches to appropriate method handler
    fn handle_request(&mut self, message: &Message) -> Result<Response> {
        let id = message.id.clone().unwrap_or(Value::Null);

        // Check if method requires initialization
        // Special cases: initialize and ping work in any state
        if !self.is_method_allowed(&message.method) {
            log::warn!(
                "Request {} rejected: server not initialized",
                message.method
            );
            return Ok(create_error_response(
                id,
                error_codes::SERVER_NOT_INITIALIZED,
                "Server not initialized. Call 'initialize' first.",
            ));
        }

        // Dispatch to method handlers
        match message.method.as_str() {
            "initialize" => self.handle_initialize(id, message),
            "ping" => self.handle_ping(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, message),
            _ => {
                // Unknown method
                log::warn!("Unknown method requested: {}", message.method);
                Ok(create_error_response(
                    id,
                    error_codes::METHOD_NOT_FOUND,
                    &format!("Method not found: {}", message.method),
                ))
            }
        }
    }

    /// Check if a method is allowed in the current state
    ///
    /// - `initialize` and `ping` are always allowed
    /// - All other methods require Ready state
    fn is_method_allowed(&self, method: &str) -> bool {
        match method {
            "initialize" | "ping" => true,
            _ => self.state == ServerState::Ready,
        }
    }

    /// Handle the `initialize` request
    ///
    /// This is the first request in the MCP protocol lifecycle.
    /// It negotiates protocol version and capabilities.
    fn handle_initialize(&mut self, id: Value, message: &Message) -> Result<Response> {
        log::info!("Handling initialize request");

        let params: InitializeParams = if let Some(params) = &message.params {
            serde_json::from_value(params.clone()).wrap_err("Invalid initialize parameters")?
        } else {
            return Ok(create_error_response(
                id,
                error_codes::INVALID_PARAMS,
                "Missing initialize parameters",
            ));
        };

        // Validate protocol version
        if !SUPPORTED_PROTOCOL_VERSIONS.contains(&params.protocol_version.as_str()) {
            log::error!(
                "Protocol version mismatch: client={}, server={:?}",
                params.protocol_version,
                SUPPORTED_PROTOCOL_VERSIONS
            );
            return Ok(create_error_response(
                id,
                error_codes::INVALID_PARAMS,
                &format!(
                    "Unsupported protocol version: {}. Server supports: {:?}",
                    params.protocol_version, SUPPORTED_PROTOCOL_VERSIONS
                ),
            ));
        }

        log::info!(
            "Client: {} {}",
            params.client_info.name,
            params.client_info.version
        );

        // Transition to initializing state
        self.state = ServerState::Initializing;

        // Build initialize result with server capabilities
        let result = InitializeResult {
            protocol_version: params.protocol_version.clone(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
                resources: None,
                prompts: None,
            },
            server_info: ServerInfo {
                name: "treesitter-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    /// Handle the `ping` request
    ///
    /// Simple health check that works in any state.
    /// Returns an empty object as the result.
    fn handle_ping(&self, id: Value) -> Result<Response> {
        log::debug!("Handling ping request");
        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({})),
            error: None,
        })
    }

    /// Handle the `tools/list` request
    ///
    /// Returns the list of all registered tools with their schemas.
    /// This allows clients to discover what tools are available.
    fn handle_tools_list(&self, id: Value) -> Result<Response> {
        log::debug!("Handling tools/list request");

        let tools = self.tool_registry.list();
        let result = ListToolsResult {
            tools: tools.into_iter().cloned().collect(),
        };

        log::debug!("Returning {} tools", result.tools.len());

        Ok(Response {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        })
    }

    /// Handle the `tools/call` request
    ///
    /// Invokes a tool with the given arguments.
    ///
    /// # Error Handling
    /// Returns JSON-RPC errors for:
    /// - Missing or invalid parameters (code -32602)
    /// - Tool not found (code -32602)
    /// - Tool execution failures (code -32603)
    fn handle_tools_call(&self, id: Value, message: &Message) -> Result<Response> {
        log::debug!("Handling tools/call request");

        // Parse and validate parameters
        let params: ToolCallParams = if let Some(params) = &message.params {
            match serde_json::from_value(params.clone()) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("Invalid tool call parameters: {e}");
                    return Ok(create_error_response(
                        id,
                        error_codes::INVALID_PARAMS,
                        &format!("Invalid tool call parameters: {e}"),
                    ));
                }
            }
        } else {
            return Ok(create_error_response(
                id,
                error_codes::INVALID_PARAMS,
                "Missing tool call parameters",
            ));
        };

        log::info!("Calling tool: {}", params.name);

        // Verify tool exists in registry
        if !self.tool_registry.has_tool(&params.name) {
            log::warn!("Tool not found: {}", params.name);
            return Ok(create_error_response(
                id,
                error_codes::INVALID_PARAMS,
                &format!("Tool not found: {}", params.name),
            ));
        }

        // Invoke the tool
        let result = match params.name.as_str() {
            "parse_file" => crate::analysis::parse_file::execute(&params.arguments),
            "file_shape" => crate::analysis::file_shape::execute(&params.arguments),
            "code_map" => crate::analysis::code_map::execute(&params.arguments),
            "find_usages" => crate::analysis::find_usages::execute(&params.arguments),
            "query_pattern" => crate::analysis::query_pattern::execute(&params.arguments),
            _ => {
                return Ok(create_error_response(
                    id,
                    error_codes::INTERNAL_ERROR,
                    &format!(
                        "Tool '{}' is registered but not yet implemented",
                        params.name
                    ),
                ));
            }
        };

        // Handle tool execution result
        match result {
            Ok(tool_result) => Ok(Response {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(serde_json::to_value(tool_result)?),
                error: None,
            }),
            Err(e) => {
                log::error!("Tool execution failed: {e}");
                Ok(Response {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(serde_json::to_value(
                        crate::mcp::types::CallToolResult::error(format!(
                            "Tool execution failed: {e}"
                        )),
                    )?),
                    error: None,
                })
            }
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
