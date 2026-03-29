use crate::client::McpClient;
use crate::error::McpError;
use crate::types::{McpConnectionInfo, McpTool, McpToolInfo, McpToolResult};
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
    ) -> Result<Vec<McpTool>, McpError> {
        let mut guard = self.servers.write().await;
        if let Some(mut old) = guard.remove(server_id) {
            let _ = old.client.shutdown().await;
        }
        let client = McpClient::new(command, args, env).await?;
        client.initialize().await?;
        let tools = client.list_tools().await?;
        guard.insert(
            server_id.to_string(),
            McpServerEntry {
                client,
                tools: tools.clone(),
                server_name: name.to_string(),
            },
        );
        Ok(tools)
    }

    pub async fn disconnect(&self, server_id: &str) -> Result<(), McpError> {
        let mut guard = self.servers.write().await;
        let mut entry = guard
            .remove(server_id)
            .ok_or(McpError::ServerNotFound(server_id.to_string()))?;
        entry.client.shutdown().await
    }

    pub async fn disconnect_all(&self) -> Result<(), McpError> {
        let mut guard = self.servers.write().await;
        for (_, mut entry) in guard.drain() {
            let _ = entry.client.shutdown().await;
        }
        Ok(())
    }

    pub async fn call_tool(
        &self,
        server_id: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let guard = self.servers.read().await;
        let entry = guard
            .get(server_id)
            .ok_or(McpError::ServerNotFound(server_id.to_string()))?;
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
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_new_empty() {
        let manager = McpManager::new();
        let connections = manager.list_connections().await;
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_manager_list_tools_empty() {
        let manager = McpManager::new();
        let tools = manager.list_tools_info().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_manager_disconnect_not_found() {
        let manager = McpManager::new();
        let result = manager.disconnect("nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ServerNotFound(id) => assert_eq!(id, "nonexistent"),
            other => panic!("expected ServerNotFound, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_manager_call_tool_not_found() {
        let manager = McpManager::new();
        let result = manager
            .call_tool("nonexistent", "tool", serde_json::json!({}))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_get_all_tools_empty() {
        let manager = McpManager::new();
        let tools = manager.get_all_tools().await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_manager_default() {
        let manager = McpManager::default();
        assert!(manager.list_connections().await.is_empty());
    }
}
