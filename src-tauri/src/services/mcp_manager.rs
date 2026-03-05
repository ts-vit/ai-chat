// MCP manager: multiple MCP servers, connect/disconnect, list tools, call tool
use crate::models::mcp::{McpConnectionInfo, McpTool, McpToolResult, McpToolInfo};
use crate::services::mcp_client::McpClient;
use std::collections::HashMap;
use tokio::sync::RwLock;

struct McpServerEntry {
    client: McpClient,
    tools: Vec<McpTool>,
    server_name: String,
}

pub struct McpManager {
    servers: RwLock<HashMap<String, McpServerEntry>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn connect(
        &self,
        server_id: &str,
        name: &str,
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Result<Vec<McpTool>, String> {
        let mut guard = self.servers.write().await;
        if let Some(mut old) = guard.remove(server_id) {
            let _ = old.client.shutdown().await;
        }
        let client = McpClient::new(command, args, env).await?;
        client.initialize().await?;
        let tools = client.list_tools().await?;
        let server_name = name.to_string();
        guard.insert(
            server_id.to_string(),
            McpServerEntry {
                client,
                tools: tools.clone(),
                server_name,
            },
        );
        Ok(tools)
    }

    #[allow(dead_code)]
    pub async fn disconnect_all(&self) -> Result<(), String> {
        let mut guard = self.servers.write().await;
        for (_, mut entry) in guard.drain() {
            let _ = entry.client.shutdown().await;
        }
        Ok(())
    }

    pub async fn disconnect(&self, server_id: &str) -> Result<(), String> {
        let mut guard = self.servers.write().await;
        let mut entry = guard.remove(server_id).ok_or("server not found")?;
        entry.client.shutdown().await
    }

    #[allow(dead_code)]
    pub async fn get_all_tools(&self) -> Vec<(String, McpTool)> {
        let guard = self.servers.read().await;
        let mut out = Vec::new();
        for (id, entry) in guard.iter() {
            for tool in &entry.tools {
                out.push((id.clone(), tool.clone()));
            }
        }
        out
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, String> {
        let guard = self.servers.read().await;
        let entry = guard.get(server_id).ok_or("server not found")?;
        entry.client.call_tool(tool_name, arguments).await
    }

    pub async fn list_connections(&self) -> Vec<McpConnectionInfo> {
        let guard = self.servers.read().await;
        guard
            .iter()
            .map(|(id, entry)| McpConnectionInfo {
                id: id.clone(),
                name: entry.server_name.clone(),
                tool_count: entry.tools.len(),
                connected: true,
            })
            .collect()
    }

    pub async fn list_tools_info(&self) -> Vec<McpToolInfo> {
        let guard = self.servers.read().await;
        let mut out = Vec::new();
        for (id, entry) in guard.iter() {
            for tool in &entry.tools {
                out.push(McpToolInfo {
                    server_id: id.clone(),
                    server_name: entry.server_name.clone(),
                    tool: tool.clone(),
                });
            }
        }
        out
    }
}
