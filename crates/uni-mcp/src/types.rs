use serde::{Deserialize, Serialize};

/// An MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Result of an MCP tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// A content block in an MCP tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// Information about a connected MCP server
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConnectionInfo {
    pub id: String,
    pub name: String,
    pub tool_count: usize,
    pub connected: bool,
}

/// A tool with its server info
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    pub server_id: String,
    pub server_name: String,
    pub tool: McpTool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_tool_deserialize() {
        let json = serde_json::json!({
            "name": "read_file",
            "description": "Read a file",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            }
        });
        let tool: McpTool = serde_json::from_value(json).unwrap();
        assert_eq!(tool.name, "read_file");
        assert_eq!(tool.description, Some("Read a file".to_string()));
    }

    #[test]
    fn test_mcp_tool_result_deserialize() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "file contents here" }
            ],
            "isError": false
        });
        let result: McpToolResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text, "file contents here");
        assert!(!result.is_error);
    }

    #[test]
    fn test_mcp_tool_result_error() {
        let json = serde_json::json!({
            "content": [
                { "type": "text", "text": "file not found" }
            ],
            "isError": true
        });
        let result: McpToolResult = serde_json::from_value(json).unwrap();
        assert!(result.is_error);
    }

    #[test]
    fn test_mcp_tool_no_description() {
        let json = serde_json::json!({
            "name": "simple_tool",
            "inputSchema": {}
        });
        let tool: McpTool = serde_json::from_value(json).unwrap();
        assert_eq!(tool.name, "simple_tool");
        assert!(tool.description.is_none());
    }

    #[test]
    fn test_mcp_connection_info_serialize() {
        let info = McpConnectionInfo {
            id: "srv1".to_string(),
            name: "Test Server".to_string(),
            tool_count: 5,
            connected: true,
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["toolCount"], 5);
    }

    #[test]
    fn test_mcp_tool_info_serialize() {
        let info = McpToolInfo {
            server_id: "srv1".to_string(),
            server_name: "Test".to_string(),
            tool: McpTool {
                name: "test_tool".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
            },
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["serverId"], "srv1");
        assert_eq!(json["serverName"], "Test");
    }
}
