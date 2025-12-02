use serde_json::json;

#[test]
fn test_initialize_request_deserialization() {
    let json_str = r#"{
        "jsonrpc":"2.0",
        "id":1,
        "method":"initialize",
        "params":{
            "protocolVersion":"2025-11-25",
            "capabilities":{},
            "clientInfo":{"name":"test-client","version":"1.0.0"}
        }
    }"#;

    let msg: treesitter_cli::mcp::json_rpc::Message = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.method, "initialize");

    // Parse params into InitializeParams
    let params: treesitter_cli::mcp::types::InitializeParams =
        serde_json::from_value(msg.params.unwrap()).unwrap();
    assert_eq!(params.protocol_version, "2025-11-25");
    assert_eq!(params.client_info.name, "test-client");
}

#[test]
fn test_initialize_response_serialization() {
    let response = treesitter_cli::mcp::types::InitializeResult {
        protocol_version: "2025-11-25".to_string(),
        capabilities: treesitter_cli::mcp::types::ServerCapabilities {
            tools: Some(treesitter_cli::mcp::types::ToolsCapability {}),
            resources: None,
            prompts: None,
        },
        server_info: treesitter_cli::mcp::types::ServerInfo {
            name: "treesitter-mcp".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["protocolVersion"], "2025-11-25");
    assert!(json["capabilities"]["tools"].is_object());
    assert_eq!(json["serverInfo"]["name"], "treesitter-mcp");
}

#[test]
fn test_tool_definition_schema() {
    let tool = treesitter_cli::mcp::types::ToolDefinition {
        name: "parse_file".to_string(),
        description: "Parse a source file using tree-sitter".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"}
            },
            "required": ["file_path"]
        }),
    };

    let json = serde_json::to_value(&tool).unwrap();
    assert_eq!(json["name"], "parse_file");
    assert_eq!(json["inputSchema"]["type"], "object");
}

#[test]
fn test_tool_call_params() {
    let json_str = r#"{
        "name": "parse_file",
        "arguments": {
            "file_path": "src/main.rs"
        }
    }"#;

    let params: treesitter_cli::mcp::types::ToolCallParams =
        serde_json::from_str(json_str).unwrap();
    assert_eq!(params.name, "parse_file");
    assert_eq!(params.arguments["file_path"], "src/main.rs");
}

#[test]
fn test_tool_result_with_text_content() {
    let result = treesitter_cli::mcp::types::CallToolResult {
        content: vec![
            treesitter_cli::mcp::types::Content::Text(
                treesitter_cli::mcp::types::TextContent {
                    type_: "text".to_string(),
                    text: "Parse result here".to_string(),
                }
            )
        ],
        is_error: false,
    };

    let json = serde_json::to_value(&result).unwrap();
    assert!(json["content"].is_array());
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["isError"], false);
}

#[test]
fn test_server_capabilities_serialization() {
    let caps = treesitter_cli::mcp::types::ServerCapabilities {
        tools: Some(treesitter_cli::mcp::types::ToolsCapability {}),
        resources: None,
        prompts: None,
    };

    let json = serde_json::to_value(&caps).unwrap();
    assert!(json["tools"].is_object());
    assert!(json["resources"].is_null());
}

#[test]
fn test_client_info_structure() {
    let info = treesitter_cli::mcp::types::ClientInfo {
        name: "claude-desktop".to_string(),
        version: "1.0.0".to_string(),
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["name"], "claude-desktop");
    assert_eq!(json["version"], "1.0.0");
}
