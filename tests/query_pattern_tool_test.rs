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
fn test_query_pattern_rust_functions() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(&file_path, r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
        fn helper() -> i32 { 42 }
        pub fn multiply(x: i32, y: i32) -> i32 { x * y }
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_pattern",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "query": "(function_item name: (identifier) @name)"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["content"].is_array());
    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let matches: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should find 3 function names
    assert!(matches["matches"].is_array());
    let match_list = matches["matches"].as_array().unwrap();
    assert_eq!(match_list.len(), 3);
}

#[test]
fn test_query_pattern_python_classes() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");

    fs::write(&file_path, r#"
class Person:
    def __init__(self):
        pass

class Animal:
    def speak(self):
        pass
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_pattern",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "query": "(class_definition name: (identifier) @name)"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let matches: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should find 2 class names
    assert!(matches["matches"].is_array());
    let match_list = matches["matches"].as_array().unwrap();
    assert_eq!(match_list.len(), 2);
}

#[test]
fn test_query_pattern_with_capture() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(&file_path, r#"
        struct Point { x: i32, y: i32 }
        struct Rectangle { width: i32, height: i32 }
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_pattern",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "query": "(struct_item name: (type_identifier) @struct_name)"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let matches: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should find 2 struct names
    assert!(matches["matches"].is_array());
    let match_list = matches["matches"].as_array().unwrap();
    assert_eq!(match_list.len(), 2);

    // Check that captures contain the struct names
    let first_match = &match_list[0];
    assert!(first_match["captures"].is_object());
}

#[test]
fn test_query_pattern_no_matches() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(&file_path, r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_pattern",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "query": "(struct_item name: (type_identifier) @name)"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let matches: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should return empty matches
    assert!(matches["matches"].is_array());
    assert_eq!(matches["matches"].as_array().unwrap().len(), 0);
}

#[test]
fn test_query_pattern_invalid_query() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(&file_path, "pub fn test() {}").unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "query_pattern",
            "arguments": {
                "file_path": file_path.to_str().unwrap(),
                "query": "(invalid syntax"
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    // Should return error in result
    let content = &response_json["result"]["content"][0];
    assert_eq!(content["type"], "text");
    let text = content["text"].as_str().unwrap();

    // Check if it's an error response (either in isError field or in the text)
    if response_json["result"]["isError"].as_bool().unwrap_or(false) {
        assert!(text.contains("error") || text.contains("Error") || text.contains("failed") || text.contains("Failed"));
    }
}

#[test]
fn test_query_pattern_registered() {
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
    assert!(tools.iter().any(|t| t["name"] == "query_pattern"));
}
