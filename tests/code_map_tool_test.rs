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
fn test_code_map_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");
    fs::write(&file_path, r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
        struct Point { x: i32, y: i32 }
        use std::fmt;
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "code_map",
            "arguments": {
                "path": temp_dir.path().to_str().unwrap(),
                "max_tokens": 1000
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["content"].is_array());
    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let map: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should contain file information
    assert!(map["files"].is_array());
    assert!(map["files"].as_array().unwrap().len() > 0);
}

#[test]
fn test_code_map_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create a simple directory structure
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    fs::write(src_dir.join("lib.rs"), r#"
        pub mod helper;
        pub fn main() {}
    "#).unwrap();

    fs::write(src_dir.join("helper.rs"), r#"
        pub fn helper_fn() -> i32 { 42 }
    "#).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "code_map",
            "arguments": {
                "path": src_dir.to_str().unwrap(),
                "max_tokens": 2000
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let map: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should contain multiple files
    assert!(map["files"].is_array());
    let files = map["files"].as_array().unwrap();
    assert!(files.len() >= 2);
}

#[test]
fn test_code_map_respects_token_limit() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    // Create a large file with many functions
    let mut content = String::new();
    for i in 0..100 {
        content.push_str(&format!("pub fn function_{i}() {{ println!(\"test\"); }}\n"));
    }
    fs::write(&file_path, content).unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "code_map",
            "arguments": {
                "path": temp_dir.path().to_str().unwrap(),
                "max_tokens": 500
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();

    // The output should be limited (rough token count check)
    // Approximate: 1 token ~ 4 characters
    assert!(text.len() < 500 * 6); // Allow some overhead
}

#[test]
fn test_code_map_registered() {
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
    assert!(tools.iter().any(|t| t["name"] == "code_map"));
}
