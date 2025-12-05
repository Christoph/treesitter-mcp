use serde_json::json;
use treesitter_mcp::mcp::server::McpServer;

/// Helper to create and initialize a test server
fn create_initialized_server() -> McpServer {
    let mut server = McpServer::new();

    // Initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });
    server.handle_message(&init_request.to_string()).unwrap();

    // Send initialized notification
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    server.handle_message(&initialized.to_string()).unwrap();

    server
}

#[test]
fn test_tools_list_method() {
    let mut server = create_initialized_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["tools"].is_array());
    let tools = response_json["result"]["tools"].as_array().unwrap();
    // Should have at least parse_file tool
    assert!(!tools.is_empty());
    assert!(tools.iter().any(|t| t["name"] == "parse_file"));
}

#[test]
fn test_tools_call_unknown_tool() {
    let mut server = create_initialized_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "nonexistent_tool",
            "arguments": {}
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should return error
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32602); // Invalid params (tool not found)
}

#[test]
fn test_tools_call_invalid_params() {
    let mut server = create_initialized_server();

    // Missing name parameter
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "arguments": {}
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should return invalid params error
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32602);
}

#[test]
fn test_tools_list_before_initialization() {
    let mut server = McpServer::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should require initialization
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32002);
}

#[test]
fn test_tools_call_before_initialization() {
    let mut server = McpServer::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "test",
            "arguments": {}
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should require initialization
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32002);
}
