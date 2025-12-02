use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn create_test_server() -> treesitter_cli::mcp::server::McpServer {
    let mut server = treesitter_cli::mcp::server::McpServer::new();
    let init = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }
    });
    server.handle_message(&init.to_string()).unwrap();
    let initialized = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    server.handle_message(&initialized.to_string()).unwrap();
    server
}

#[test]
fn test_file_shape_rust() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");
    fs::write(&file_path, r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
        struct Point { x: i32 }
        use std::fmt;
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "file_shape",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "include_deps": false
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["content"].is_array());
    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let shape: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(shape["functions"].is_array());
    assert!(shape["structs"].is_array());
    assert!(shape["imports"].is_array());
}

#[test]
fn test_file_shape_python() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    fs::write(&file_path, r#"
def hello():
    pass

class MyClass:
    def method(self):
        pass

import os
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "file_shape",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "include_deps": false
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let shape: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(shape["functions"].is_array());
    assert!(shape["classes"].is_array());
}

#[test]
fn test_file_shape_registered() {
    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let tools = response_json["result"]["tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "file_shape"));
}
