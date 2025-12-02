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
fn test_file_shape_rust_with_deps() {
    let temp_dir = TempDir::new().unwrap();

    // Create a minimal Cargo.toml to mark the project root
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(
        &cargo_toml,
        r#"[package]
name = "test_project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    // Create a simple module structure: src/lib.rs -> mod utils; and src/utils.rs
    let src_dir = temp_dir.path().join("src");
    std::fs::create_dir(&src_dir).unwrap();

    let lib_rs = src_dir.join("lib.rs");
    fs::write(
        &lib_rs,
        r#"
        mod utils;

        use std::fmt;

        pub fn call_add() -> i32 {
            utils::add(1, 2)
        }
    "#,
    )
    .unwrap();

    let utils_rs = src_dir.join("utils.rs");
    fs::write(
        &utils_rs,
        r#"
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }
    "#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "file_shape",
            "arguments": {
                "file_path": lib_rs.to_str().unwrap(),
                "include_deps": true
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let shape: serde_json::Value = serde_json::from_str(text).unwrap();

    // Root file shape checks
    assert!(shape["functions"].is_array());
    assert!(shape["imports"].is_array());

    // Dependencies should be present as a tree of shapes
    let deps = shape["dependencies"].as_array().expect("dependencies should be an array");
    assert_eq!(deps.len(), 1, "expected exactly one project dependency");

    let dep = &deps[0];
    let dep_path = dep["path"].as_str().expect("dependency should have a path");
    assert!(
        dep_path.ends_with("src/utils.rs"),
        "dependency path should point to utils.rs, got {}",
        dep_path
    );

    // Dependency should also expose its own functions
    let dep_functions = dep["functions"]
        .as_array()
        .expect("dependency should have functions array");
    assert!(
        dep_functions
            .iter()
            .any(|f| f["name"] == "add"),
        "dependency should contain the 'add' function"
    );
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
fn test_file_shape_python_with_deps() {
    let temp_dir = TempDir::new().unwrap();

    let main_py = temp_dir.path().join("main.py");
    let utils_py = temp_dir.path().join("utils.py");

    fs::write(
        &main_py,
        r#"
import utils

def run():
    return utils.add(1, 2)
"#,
    )
    .unwrap();

    fs::write(
        &utils_py,
        r#"
def add(a, b):
    return a + b
"#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "file_shape",
            "arguments": {
                "file_path": main_py.to_str().unwrap(),
                "include_deps": true
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let shape: serde_json::Value = serde_json::from_str(text).unwrap();

    // Root file shape checks
    assert!(shape["functions"].is_array());

    // Dependencies should contain utils.py
    let deps = shape["dependencies"].as_array().expect("dependencies should be an array");
    assert_eq!(deps.len(), 1, "expected exactly one python dependency");

    let dep = &deps[0];
    let dep_path = dep["path"].as_str().expect("dependency should have a path");
    assert!(
        dep_path.ends_with("utils.py"),
        "dependency path should point to utils.py, got {}",
        dep_path
    );

    let dep_functions = dep["functions"]
        .as_array()
        .expect("dependency should have functions array");
    assert!(
        dep_functions
            .iter()
            .any(|f| f["name"] == "add"),
        "dependency should contain the 'add' function"
    );
}

#[test]
fn test_file_shape_js_with_deps() {
    let temp_dir = TempDir::new().unwrap();

    let main_js = temp_dir.path().join("main.js");
    let utils_js = temp_dir.path().join("utils.js");

    fs::write(
        &main_js,
        r#"
import { add } from "./utils.js";

export function run() {
    return add(1, 2);
}
"#,
    )
    .unwrap();

    fs::write(
        &utils_js,
        r#"
export function add(a, b) {
    return a + b;
}
"#,
    )
    .unwrap();

    let mut server = create_test_server();
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "file_shape",
            "arguments": {
                "file_path": main_js.to_str().unwrap(),
                "include_deps": true
            }
        }
    });

    let response = server.handle_message(&request.to_string()).unwrap();
    let response_json: serde_json::Value = serde_json::from_str(&response).unwrap();

    let text = response_json["result"]["content"][0]["text"].as_str().unwrap();
    let shape: serde_json::Value = serde_json::from_str(text).unwrap();

    // Root file shape checks
    assert!(shape["functions"].is_array());

    // Dependencies should contain utils.js
    let deps = shape["dependencies"].as_array().expect("dependencies should be an array");
    assert_eq!(deps.len(), 1, "expected exactly one JS dependency");

    let dep = &deps[0];
    let dep_path = dep["path"].as_str().expect("dependency should have a path");
    assert!(
        dep_path.ends_with("utils.js"),
        "dependency path should point to utils.js, got {}",
        dep_path
    );

    let dep_functions = dep["functions"]
        .as_array()
        .expect("dependency should have functions array");
    assert!(
        dep_functions
            .iter()
            .any(|f| f["name"] == "add"),
        "dependency should contain the 'add' function"
    );
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
