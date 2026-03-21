// MCP manager: multiple MCP servers, connect/disconnect, list tools, call tool
use crate::models::mcp::{McpConnectionInfo, McpTool, McpToolResult, McpToolInfo};
use crate::services::mcp_client::McpClient;
use crate::services::builtin_mcp_client::BuiltinMcpClient;
use crate::services::builtin_fs_server::BuiltinFsServer;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

enum McpClientKind {
    External(McpClient),
    Builtin(BuiltinMcpClient),
}

struct McpServerEntry {
    client: McpClientKind,
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
        if let Some(old) = guard.remove(server_id) {
            match old.client {
                McpClientKind::External(mut c) => { let _ = c.shutdown().await; }
                McpClientKind::Builtin(c) => { let _ = c.shutdown().await; }
            }
        }
        let client = McpClient::new(command, args, env).await?;
        client.initialize().await?;
        let tools = client.list_tools().await?;
        let server_name = name.to_string();
        guard.insert(
            server_id.to_string(),
            McpServerEntry {
                client: McpClientKind::External(client),
                tools: tools.clone(),
                server_name,
            },
        );
        Ok(tools)
    }

    pub async fn connect_builtin(
        &self,
        server_id: &str,
        name: &str,
        server: Arc<BuiltinFsServer>,
    ) -> Result<Vec<McpTool>, String> {
        let mut guard = self.servers.write().await;
        if let Some(old) = guard.remove(server_id) {
            match old.client {
                McpClientKind::External(mut c) => { let _ = c.shutdown().await; }
                McpClientKind::Builtin(c) => { let _ = c.shutdown().await; }
            }
        }
        let client = BuiltinMcpClient::new(server);
        client.initialize().await?;
        let tools = client.list_tools().await?;
        let server_name = name.to_string();
        guard.insert(
            server_id.to_string(),
            McpServerEntry {
                client: McpClientKind::Builtin(client),
                tools: tools.clone(),
                server_name,
            },
        );
        Ok(tools)
    }

    #[allow(dead_code)]
    pub async fn disconnect_all(&self) -> Result<(), String> {
        let mut guard = self.servers.write().await;
        for (_, entry) in guard.drain() {
            match entry.client {
                McpClientKind::External(mut c) => { let _ = c.shutdown().await; }
                McpClientKind::Builtin(c) => { let _ = c.shutdown().await; }
            }
        }
        Ok(())
    }

    pub async fn disconnect(&self, server_id: &str) -> Result<(), String> {
        let mut guard = self.servers.write().await;
        let entry = guard.remove(server_id).ok_or("server not found")?;
        match entry.client {
            McpClientKind::External(mut c) => c.shutdown().await,
            McpClientKind::Builtin(c) => c.shutdown().await,
        }
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
        match &entry.client {
            McpClientKind::External(c) => c.call_tool(tool_name, arguments).await,
            McpClientKind::Builtin(c) => c.call_tool(tool_name, arguments).await,
        }
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
