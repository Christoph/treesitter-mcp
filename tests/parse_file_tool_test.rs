use serde_json::json;
use std::fs;
use tempfile::TempDir;
use treesitter_mcp::mcp::server::McpServer;

fn create_test_server() -> McpServer {
    let mut server = McpServer::new();

    // Initialize
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
fn test_parse_file_tool_rust() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, "fn main() { let x = 42; }").unwrap();

    let mut server = create_test_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "parse_file",
            "arguments": {
                "file_path": file_path.to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"].is_object());
    let content = &response_json["result"]["content"];
    assert!(content.is_array());
    assert!(content[0]["type"] == "text");

    let text = content[0]["text"].as_str().unwrap();
    assert!(text.contains("source_file"));
    assert!(text.contains("function_item"));
}

#[test]
fn test_parse_file_not_found() {
    let mut server = create_test_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "parse_file",
            "arguments": {
                "file_path": "/nonexistent/file.rs"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["isError"].as_bool().unwrap());
}

#[test]
fn test_parse_file_unsupported_language() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "plain text").unwrap();

    let mut server = create_test_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "parse_file",
            "arguments": {
                "file_path": file_path.to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["isError"].as_bool().unwrap());
}

#[test]
fn test_parse_file_with_syntax_errors() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid.rs");
    fs::write(&file_path, "fn main( { }").unwrap();

    let mut server = create_test_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "parse_file",
            "arguments": {
                "file_path": file_path.to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should still parse but tree contains errors
    assert!(response_json["result"]["content"].is_array());
    let text = response_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    // Tree-sitter marks syntax errors as ERROR or MISSING nodes
    assert!(text.contains("ERROR") || text.contains("MISSING"));
}

#[test]
fn test_parse_file_registered_in_tools_list() {
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
    assert!(tools.iter().any(|t| t["name"] == "parse_file"));
}
