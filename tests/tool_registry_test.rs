use serde_json::json;

#[test]
fn test_register_tool() {
    let mut registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();

    let tool = treesitter_cli::mcp::types::ToolDefinition {
        name: "test_tool".to_string(),
        description: "A test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "arg1": {"type": "string"}
            }
        }),
    };

    registry.register(tool);
    assert_eq!(registry.list().len(), 1);
}

#[test]
fn test_list_tools() {
    let mut registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();

    registry.register(create_tool("parse_file", "Parse a file"));
    registry.register(create_tool("file_shape", "Get file shape"));
    registry.register(create_tool("code_map", "Generate code map"));

    let tools = registry.list();
    assert_eq!(tools.len(), 3);
    assert!(tools.iter().any(|t| t.name == "parse_file"));
    assert!(tools.iter().any(|t| t.name == "file_shape"));
    assert!(tools.iter().any(|t| t.name == "code_map"));
}

#[test]
fn test_get_tool_by_name() {
    let mut registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();
    registry.register(create_tool("parse_file", "Parse a file"));

    let tool = registry.get("parse_file");
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().name, "parse_file");
}

#[test]
fn test_tool_not_found() {
    let registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();
    let tool = registry.get("nonexistent");
    assert!(tool.is_none());
}

#[test]
fn test_register_duplicate_tool_replaces() {
    let mut registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();

    registry.register(create_tool("test", "First version"));
    registry.register(create_tool("test", "Second version"));

    let tools = registry.list();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].description, "Second version");
}

#[test]
fn test_empty_registry() {
    let registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();
    assert_eq!(registry.list().len(), 0);
}

#[test]
fn test_has_tool() {
    let mut registry = treesitter_cli::mcp::tool_registry::ToolRegistry::new();

    assert!(!registry.has_tool("parse_file"));

    registry.register(create_tool("parse_file", "Parse a file"));

    assert!(registry.has_tool("parse_file"));
    assert!(!registry.has_tool("other_tool"));
}

// Helper function
fn create_tool(name: &str, description: &str) -> treesitter_cli::mcp::types::ToolDefinition {
    treesitter_cli::mcp::types::ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
    }
}
