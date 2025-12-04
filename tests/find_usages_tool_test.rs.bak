use serde_json::json;
use std::fs;
use tempfile::TempDir;
use treesitter_mcp::mcp::server::McpServer;

fn create_test_server() -> McpServer {
    let mut server = McpServer::new();
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
fn test_find_usages_rust() {
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("lib.rs");
    let file2 = temp_dir.path().join("helper.rs");

    fs::write(
        &file1,
        r#"
        pub fn add(a: i32, b: i32) -> i32 {
            helper_fn() + a + b
        }

        fn test() {
            let result = helper_fn();
        }
    "#,
    )
    .unwrap();

    fs::write(
        &file2,
        r#"
        pub fn helper_fn() -> i32 { 42 }
    "#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_usages",
            "arguments": {
                "symbol": "helper_fn",
                "path": temp_dir.path().to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    assert!(response_json["result"]["content"].is_array());
    let text = response_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let usages: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should find usages in both files
    assert!(usages["usages"].is_array());
    let usage_list = usages["usages"].as_array().unwrap();
    assert!(usage_list.len() >= 2); // At least 2 usages
}

#[test]
fn test_find_usages_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");

    fs::write(
        &file_path,
        r#"
def calculate(x):
    return x * 2

def main():
    result = calculate(5)
    value = calculate(10)
    return calculate(result)
    "#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_usages",
            "arguments": {
                "symbol": "calculate",
                "path": file_path.to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let usages: serde_json::Value = serde_json::from_str(text).unwrap();

    assert!(usages["usages"].is_array());
    let usage_list = usages["usages"].as_array().unwrap();
    // Should find 3 calls to calculate
    assert!(usage_list.len() >= 3);
}

#[test]
fn test_find_usages_not_found() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("lib.rs");

    fs::write(
        &file_path,
        r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
    "#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "find_usages",
            "arguments": {
                "symbol": "nonexistent_function",
                "path": file_path.to_str().unwrap()
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    let usages: serde_json::Value = serde_json::from_str(text).unwrap();

    // Should return empty usages array
    assert!(usages["usages"].is_array());
    assert_eq!(usages["usages"].as_array().unwrap().len(), 0);
}

#[test]
fn test_find_usages_registered() {
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
    assert!(tools.iter().any(|t| t["name"] == "find_usages"));
}
