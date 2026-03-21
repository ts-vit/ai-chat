// Thin client wrapping BuiltinFsServer, same API shape as McpClient
use crate::models::mcp::{McpTool, McpToolResult};
use crate::services::builtin_fs_server::BuiltinFsServer;
use serde_json::json;
use std::sync::Arc;

pub struct BuiltinMcpClient {
    server: Arc<BuiltinFsServer>,
}

impl BuiltinMcpClient {
    pub fn new(server: Arc<BuiltinFsServer>) -> Self {
        Self { server }
    }

    pub async fn initialize(&self) -> Result<(), String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "ai-chat", "version": "0.1.0" }
            }
        });
        let response = self.server.handle_jsonrpc(request).await;
        if response.get("error").is_some() {
            return Err(format!("Initialize failed: {}", response));
        }
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        let response = self.server.handle_jsonrpc(request).await;
        let result = response
            .get("result")
            .ok_or("Missing result from tools/list")?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or("Invalid tools/list result")?;
        let list: Vec<McpTool> = serde_json::from_value(serde_json::Value::Array(tools.clone()))
            .map_err(|e| e.to_string())?;
        Ok(list)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, String> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        let response = self.server.handle_jsonrpc(request).await;
        let result = response
            .get("result")
            .ok_or("Missing result from tools/call")?;
        let out: McpToolResult =
            serde_json::from_value(result.clone()).map_err(|e| e.to_string())?;
        Ok(out)
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        // No-op for built-in server
        Ok(())
    }
}
