use serde_json::json;

#[test]
fn test_parse_json_rpc_request() {
    let json_str = r#"{"jsonrpc":"2.0","id":1,"method":"test","params":{}}"#;
    let request = treesitter_cli::mcp::json_rpc::parse_message(json_str).unwrap();
    assert_eq!(request.id, Some(json!(1)));
    assert_eq!(request.method, "test");
}

#[test]
fn test_serialize_json_rpc_response() {
    let response = treesitter_cli::mcp::json_rpc::Response {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({"ok": true})),
        error: None,
    };
    let json_str = treesitter_cli::mcp::json_rpc::serialize_response(&response);
    assert!(!json_str.contains('\n')); // compact JSON
    assert!(json_str.contains("\"ok\""));
}

#[test]
fn test_parse_json_rpc_notification() {
    let json_str = r#"{"jsonrpc":"2.0","method":"notify","params":{}}"#;
    let msg = treesitter_cli::mcp::json_rpc::parse_message(json_str).unwrap();
    assert!(msg.id.is_none()); // notifications have no id
}

#[test]
fn test_error_response_format() {
    let error = treesitter_cli::mcp::json_rpc::create_error_response(
        json!(1),
        -32601,
        "Method not found",
    );
    assert_eq!(error.error.as_ref().unwrap().code, -32601);
    assert_eq!(error.error.as_ref().unwrap().message, "Method not found");
}
