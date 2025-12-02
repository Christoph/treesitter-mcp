use serde_json::json;

/// Helper to create a test server
fn create_test_server() -> treesitter_cli::mcp::server::McpServer {
    treesitter_cli::mcp::server::McpServer::new()
}

#[test]
fn test_initialization_sequence() {
    let mut server = create_test_server();

    // Server should start uninitialized
    assert!(!server.is_initialized());

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = server.handle_message(&init_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Check initialize response
    assert_eq!(response_json["result"]["protocolVersion"], "2025-11-25");
    assert!(response_json["result"]["capabilities"]["tools"].is_object());
    assert_eq!(response_json["result"]["serverInfo"]["name"], "treesitter-mcp");

    // Server should still not be fully initialized (needs initialized notification)
    assert!(!server.is_initialized());

    // Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });

    server.handle_message(&initialized_notification.to_string()).unwrap();

    // Now server should be initialized
    assert!(server.is_initialized());
}

#[test]
fn test_reject_requests_before_initialization() {
    let mut server = create_test_server();

    // Try to call tools/list before initialize
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = server.handle_message(&tools_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should return error
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32002); // Server not initialized
}

#[test]
fn test_ping_method() {
    let mut server = create_test_server();

    // Initialize the server first
    initialize_server(&mut server);

    // Send ping request
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {}
    });

    let response = server.handle_message(&ping_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should succeed
    assert!(response_json["result"].is_object());
    assert!(response_json["error"].is_null());
}

#[test]
fn test_ping_before_initialization() {
    let mut server = create_test_server();

    // Ping should work even before initialization
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "ping",
        "params": {}
    });

    let response = server.handle_message(&ping_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should succeed
    assert!(response_json["result"].is_object());
}

#[test]
fn test_unknown_method_error() {
    let mut server = create_test_server();
    initialize_server(&mut server);

    let unknown_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "unknown/method",
        "params": {}
    });

    let response = server.handle_message(&unknown_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should return method not found error
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32601);
}

#[test]
fn test_invalid_json_error() {
    let mut server = create_test_server();

    let response = server.handle_message("not valid json");
    assert!(response.is_err());
}

#[test]
fn test_notification_no_response() {
    let mut server = create_test_server();

    // Notifications should not generate responses
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/test",
        "params": {}
    });

    let response = server.handle_message(&notification.to_string()).unwrap();
    // Empty response for notifications
    assert_eq!(response, "");
}

#[test]
fn test_initialization_with_older_version() {
    let mut server = create_test_server();

    // Send initialize request with older version
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = server.handle_message(&init_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Check initialize response
    // It should succeed (no error)
    assert!(response_json["error"].is_null());
    assert!(response_json["result"].is_object());
    // We return the negotiated version
    assert_eq!(response_json["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn test_initialization_with_2024_version() {
    let mut server = create_test_server();

    // Send initialize request with 2024 version
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }
    });

    let response = server.handle_message(&init_request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Check initialize response
    assert!(response_json["error"].is_null());
    assert!(response_json["result"].is_object());
    // We return the negotiated version
    assert_eq!(response_json["result"]["protocolVersion"], "2024-11-05");
}

// Helper function to initialize a server
fn initialize_server(server: &mut treesitter_cli::mcp::server::McpServer) {
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

    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    server.handle_message(&initialized_notification.to_string()).unwrap();
}
