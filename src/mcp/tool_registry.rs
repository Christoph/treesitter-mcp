use std::collections::HashMap;
use crate::mcp::types::ToolDefinition;

/// Registry for MCP tools
///
/// The tool registry maintains the collection of available tools that can be
/// invoked by MCP clients. It provides methods to register tools, list them,
/// and look them up by name.
///
/// # Example
/// ```
/// use treesitter_cli::mcp::tool_registry::ToolRegistry;
/// use treesitter_cli::mcp::types::ToolDefinition;
/// use serde_json::json;
///
/// let mut registry = ToolRegistry::new();
///
/// let tool = ToolDefinition {
///     name: "parse_file".to_string(),
///     description: "Parse a source file".to_string(),
///     input_schema: json!({"type": "object"}),
/// };
///
/// registry.register(tool);
/// assert_eq!(registry.list().len(), 1);
/// ```
pub struct ToolRegistry {
    /// Map of tool name to tool definition
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Create a new empty tool registry
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// assert_eq!(registry.list().len(), 0);
    /// ```
    pub fn new() -> Self {
        log::debug!("Creating new tool registry");
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool in the registry
    ///
    /// If a tool with the same name already exists, it will be replaced.
    /// This allows tools to be updated at runtime if needed.
    ///
    /// # Arguments
    /// * `tool` - The tool definition to register
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    /// use treesitter_cli::mcp::types::ToolDefinition;
    /// use serde_json::json;
    ///
    /// let mut registry = ToolRegistry::new();
    /// let tool = ToolDefinition {
    ///     name: "test".to_string(),
    ///     description: "Test tool".to_string(),
    ///     input_schema: json!({"type": "object"}),
    /// };
    ///
    /// registry.register(tool);
    /// assert!(registry.has_tool("test"));
    /// ```
    pub fn register(&mut self, tool: ToolDefinition) {
        log::info!("Registering tool: {}", tool.name);
        self.tools.insert(tool.name.clone(), tool);
    }

    /// List all registered tools
    ///
    /// Returns a vector of references to all tool definitions.
    /// The order is not guaranteed.
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// let tools = registry.list();
    /// assert_eq!(tools.len(), 0);
    /// ```
    pub fn list(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Get a tool by name
    ///
    /// Returns a reference to the tool definition if it exists,
    /// or None if no tool with that name is registered.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to retrieve
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    /// use treesitter_cli::mcp::types::ToolDefinition;
    /// use serde_json::json;
    ///
    /// let mut registry = ToolRegistry::new();
    /// let tool = ToolDefinition {
    ///     name: "test".to_string(),
    ///     description: "Test".to_string(),
    ///     input_schema: json!({"type": "object"}),
    /// };
    /// registry.register(tool);
    ///
    /// assert!(registry.get("test").is_some());
    /// assert!(registry.get("nonexistent").is_none());
    /// ```
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// Check if a tool exists in the registry
    ///
    /// Returns true if a tool with the given name is registered,
    /// false otherwise.
    ///
    /// # Arguments
    /// * `name` - The name of the tool to check
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// assert!(!registry.has_tool("test"));
    /// ```
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get the number of registered tools
    ///
    /// # Example
    /// ```
    /// use treesitter_cli::mcp::tool_registry::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// assert_eq!(registry.count(), 0);
    /// ```
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
