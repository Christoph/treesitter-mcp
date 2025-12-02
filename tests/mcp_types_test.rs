use serde_json::json;
use treesitter_mcp::mcp::{json_rpc::Message, types};

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

    let msg: Message = serde_json::from_str(json_str).unwrap();
    assert_eq!(msg.method, "initialize");

    // Parse params into InitializeParams
    let params: types::InitializeParams = serde_json::from_value(msg.params.unwrap()).unwrap();
    assert_eq!(params.protocol_version, "2025-11-25");
    assert_eq!(params.client_info.name, "test-client");
}

#[test]
fn test_initialize_response_serialization() {
    let response = types::InitializeResult {
        protocol_version: "2025-11-25".to_string(),
        capabilities: types::ServerCapabilities {
            tools: Some(types::ToolsCapability {}),
            resources: None,
            prompts: None,
        },
        server_info: types::ServerInfo {
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
    let tool = types::ToolDefinition {
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

    let params: types::ToolCallParams = serde_json::from_str(json_str).unwrap();
    assert_eq!(params.name, "parse_file");
    assert_eq!(params.arguments["file_path"], "src/main.rs");
}

#[test]
fn test_tool_result_with_text_content() {
    let result = types::CallToolResult {
        content: vec![types::Content::Text(types::TextContent {
            type_: "text".to_string(),
            text: "Parse result here".to_string(),
        })],
        is_error: false,
    };

    let json = serde_json::to_value(&result).unwrap();
    assert!(json["content"].is_array());
    assert_eq!(json["content"][0]["type"], "text");
    assert_eq!(json["isError"], false);
}

#[test]
fn test_server_capabilities_serialization() {
    let caps = types::ServerCapabilities {
        tools: Some(types::ToolsCapability {}),
        resources: None,
        prompts: None,
    };

    let json = serde_json::to_value(&caps).unwrap();
    assert!(json["tools"].is_object());
    assert!(json["resources"].is_null());
}

#[test]
fn test_client_info_structure() {
    let info = types::ClientInfo {
        name: "claude-desktop".to_string(),
        version: "1.0.0".to_string(),
    };

    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["name"], "claude-desktop");
    assert_eq!(json["version"], "1.0.0");
}
