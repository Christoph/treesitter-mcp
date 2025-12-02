use serde_json::json;
use std::io::Cursor;
use treesitter_mcp::mcp::{io, json_rpc};

#[test]
fn test_read_message_from_stdin() {
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test\",\"params\":{}}\n";
    let mut reader = Cursor::new(&input[..]);
    let msg = io::read_message(&mut reader).unwrap();
    assert_eq!(msg.method, "test");
    assert_eq!(msg.id, Some(json!(1)));
}

#[test]
fn test_write_message_to_stdout() {
    let mut output = Vec::new();
    let response = json_rpc::Response {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({"status": "ok"})),
        error: None,
    };
    io::write_message(&mut output, &response).unwrap();

    // Should end with newline
    assert!(output.ends_with(b"\n"));

    // Should be compact JSON (no extra spaces)
    let output_str = String::from_utf8(output).unwrap();
    assert!(!output_str.contains("  ")); // no double spaces
    assert!(output_str.contains("\"status\""));
}

#[test]
fn test_handle_invalid_json() {
    let input = b"not valid json\n";
    let mut reader = Cursor::new(&input[..]);
    let result = io::read_message(&mut reader);
    assert!(result.is_err());
}

#[test]
fn test_read_multiple_messages() {
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"test1\",\"params\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"test2\",\"params\":{}}\n";
    let mut reader = Cursor::new(&input[..]);

    let msg1 = io::read_message(&mut reader).unwrap();
    assert_eq!(msg1.method, "test1");

    let msg2 = io::read_message(&mut reader).unwrap();
    assert_eq!(msg2.method, "test2");
}

#[test]
fn test_write_response_with_no_spaces() {
    let mut output = Vec::new();
    let response = json_rpc::Response {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({})),
        error: None,
    };
    io::write_message(&mut output, &response).unwrap();

    let output_str = String::from_utf8(output).unwrap();
    // Compact JSON should not have spaces after colons or commas
    assert!(!output_str.contains(": "));
    assert!(!output_str.contains(", "));
}
