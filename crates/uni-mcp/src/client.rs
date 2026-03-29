use crate::error::McpError;
use crate::types::{McpTool, McpToolResult};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, RwLock};

/// Configuration for MCP client
pub struct McpClientConfig {
    /// RPC request timeout in seconds (default: 30)
    pub timeout_secs: u64,
    /// MCP protocol version (default: "2024-11-05")
    pub protocol_version: String,
    /// Client name sent during initialize (default: "uni-mcp")
    pub client_name: String,
    /// Client version sent during initialize (default: "0.1.0")
    pub client_version: String,
}

impl Default for McpClientConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            protocol_version: "2024-11-05".to_string(),
            client_name: "uni-mcp".to_string(),
            client_version: "0.1.0".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

pub(crate) fn windows_command(command: String) -> String {
    if cfg!(windows) {
        match command.as_str() {
            "npx" => "npx.cmd".to_string(),
            "npm" => "npm.cmd".to_string(),
            "node" => command,
            "uvx" => "uvx.cmd".to_string(),
            "pnpm" => "pnpm.cmd".to_string(),
            _ => command,
        }
    } else {
        command
    }
}

/// Runs the stdout reader loop; supports both JSON-per-line and
/// Content-Length framed (LSP-style) transports.
async fn run_reader(
    stdout: ChildStdout,
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<JsonRpcResponse, String>>>>>,
) {
    let mut reader = BufReader::new(stdout);
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        let n = match reader.read_line(&mut line_buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                log::debug!("MCP reader: read error: {}", e);
                break;
            }
        };

        let trimmed = line_buf.trim();
        if trimmed.is_empty() {
            continue;
        }

        let json_str: String =
            if trimmed.starts_with("Content-Length:") || trimmed.starts_with("content-length:") {
                // LSP-style: parse content length, skip blank line, read exact body
                let len_part = trimmed.splitn(2, ':').nth(1).unwrap_or("").trim();
                let content_len: usize = match len_part.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("MCP reader: bad Content-Length value {:?}: {}", len_part, e);
                        continue;
                    }
                };

                // Skip remaining headers until empty line
                loop {
                    line_buf.clear();
                    match reader.read_line(&mut line_buf).await {
                        Ok(0) => {
                            log::debug!("MCP reader: EOF while reading headers");
                            return;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            log::debug!("MCP reader: read error in headers: {}", e);
                            return;
                        }
                    }
                    if line_buf.trim().is_empty() {
                        break;
                    }
                }

                let mut body = vec![0u8; content_len];
                if let Err(e) = reader.read_exact(&mut body).await {
                    log::debug!(
                        "MCP reader: read_exact error ({} bytes): {}",
                        content_len,
                        e
                    );
                    break;
                }
                match String::from_utf8(body) {
                    Ok(s) => s,
                    Err(e) => {
                        log::debug!("MCP reader: body is not valid UTF-8: {}", e);
                        continue;
                    }
                }
            } else if trimmed.starts_with('{') {
                trimmed.to_string()
            } else {
                log::debug!(
                    "MCP reader: skip non-JSON line: {:?}",
                    &line_buf[..n.min(200)]
                );
                continue;
            };

        log::debug!("MCP stdout raw: {}", &json_str[..json_str.len().min(500)]);

        let value: serde_json::Value = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                log::debug!("MCP reader: invalid JSON: {}", e);
                continue;
            }
        };
        let id = match value.get("id").and_then(|v| v.as_u64()) {
            Some(n) => n,
            None => continue,
        };
        let response = if value.get("error").is_some() {
            let err: JsonRpcError = match serde_json::from_value(value["error"].clone()) {
                Ok(e) => e,
                Err(_) => continue,
            };
            JsonRpcResponse {
                result: None,
                error: Some(err),
            }
        } else {
            JsonRpcResponse {
                result: value.get("result").cloned(),
                error: None,
            }
        };
        if let Some(tx) = pending.write().await.remove(&id) {
            let _ = tx.send(Ok(response));
        }
    }
}

pub struct McpClient {
    child: Option<Child>,
    stdin: tokio::sync::Mutex<Option<ChildStdin>>,
    #[allow(dead_code)]
    reader_handle: tokio::task::JoinHandle<()>,
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<JsonRpcResponse, String>>>>>,
    next_id: AtomicU64,
    config: McpClientConfig,
}

impl McpClient {
    /// Create a new MCP client with default config
    pub async fn new(
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    ) -> Result<Self, McpError> {
        Self::with_config(command, args, env, McpClientConfig::default()).await
    }

    /// Create a new MCP client with custom config
    pub async fn with_config(
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        config: McpClientConfig,
    ) -> Result<Self, McpError> {
        let command = windows_command(command);
        let mut cmd = Command::new(&command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if !env.is_empty() {
            cmd.envs(env);
        }
        let mut child = cmd.spawn().map_err(|e| {
            log::error!("MCP spawn failed: {}", e);
            McpError::Spawn(e.to_string())
        })?;
        let stdin = child.stdin.take().ok_or(McpError::Io("failed to take stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(McpError::Io("failed to take stdout".to_string()))?;
        let pending: Arc<RwLock<HashMap<_, _>>> = Arc::new(RwLock::new(HashMap::new()));
        let reader_handle = tokio::spawn(run_reader(stdout, Arc::clone(&pending)));
        Ok(Self {
            child: Some(child),
            stdin: tokio::sync::Mutex::new(Some(stdin)),
            reader_handle,
            pending,
            next_id: AtomicU64::new(1),
            config,
        })
    }

    async fn send_notification(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), McpError> {
        let mut msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
        });
        if let Some(p) = params {
            msg["params"] = p;
        }
        log::debug!("MCP notification: method={}", method);
        let line =
            serde_json::to_string(&msg).map_err(|e| McpError::Serde(e.to_string()))? + "\n";
        let mut guard = self.stdin.lock().await;
        let stdin = guard
            .as_mut()
            .ok_or(McpError::Io("stdin closed".to_string()))?;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| McpError::Io(e.to_string()))?;
        Ok(())
    }

    async fn request(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<JsonRpcResponse, McpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);
        let mut msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
        });
        if let Some(p) = params {
            msg["params"] = p;
        }
        let line =
            serde_json::to_string(&msg).map_err(|e| McpError::Serde(e.to_string()))? + "\n";
        log::debug!("MCP request: method={} id={}", method, id);
        {
            let mut guard = self.stdin.lock().await;
            let stdin = guard
                .as_mut()
                .ok_or(McpError::Io("stdin closed".to_string()))?;
            stdin
                .write_all(line.as_bytes())
                .await
                .map_err(|e| McpError::Io(e.to_string()))?;
            stdin
                .flush()
                .await
                .map_err(|e| McpError::Io(e.to_string()))?;
        }
        let timeout_secs = self.config.timeout_secs;
        let response = tokio::time::timeout(Duration::from_secs(timeout_secs), rx)
            .await
            .map_err(|_| {
                log::debug!("MCP request timeout: method={} id={}", method, id);
                McpError::Timeout {
                    method: method.to_string(),
                    timeout_secs,
                }
            })?
            .map_err(|_| McpError::Io("channel closed".to_string()))?
            .map_err(|e| McpError::Other(e))?;
        log::debug!(
            "MCP response: id={} has_result={} has_error={}",
            id,
            response.result.is_some(),
            response.error.is_some()
        );
        if let Some(ref err) = response.error {
            return Err(McpError::Rpc {
                code: err.code,
                message: err.message.clone(),
            });
        }
        Ok(response)
    }

    pub async fn initialize(&self) -> Result<(), McpError> {
        let params = serde_json::json!({
            "protocolVersion": self.config.protocol_version,
            "capabilities": {},
            "clientInfo": {
                "name": self.config.client_name,
                "version": self.config.client_version,
            }
        });
        self.request("initialize", Some(params)).await?;
        self.send_notification("notifications/initialized", None)
            .await?;
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        let response = self.request("tools/list", None).await?;
        let result = response
            .result
            .ok_or(McpError::Other("missing result".to_string()))?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or(McpError::Other("invalid tools/list result".to_string()))?;
        let list: Vec<McpTool> = serde_json::from_value(serde_json::Value::Array(tools.clone()))
            .map_err(|e| McpError::Serde(e.to_string()))?;
        Ok(list)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let params = serde_json::json!({ "name": name, "arguments": arguments });
        let response = self.request("tools/call", Some(params)).await?;
        let result = response
            .result
            .ok_or(McpError::Other("missing result".to_string()))?;
        let out: McpToolResult =
            serde_json::from_value(result).map_err(|e| McpError::Serde(e.to_string()))?;
        Ok(out)
    }

    pub async fn shutdown(&mut self) -> Result<(), McpError> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
            log::debug!("MCP process killed");
        }
        {
            let _ = self.stdin.lock().await.take();
        }
        self.pending.write().await.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_command_npx() {
        let result = windows_command("npx".to_string());
        if cfg!(windows) {
            assert_eq!(result, "npx.cmd");
        } else {
            assert_eq!(result, "npx");
        }
    }

    #[test]
    fn test_windows_command_node() {
        let result = windows_command("node".to_string());
        assert_eq!(result, "node");
    }

    #[test]
    fn test_windows_command_npm() {
        let result = windows_command("npm".to_string());
        if cfg!(windows) {
            assert_eq!(result, "npm.cmd");
        } else {
            assert_eq!(result, "npm");
        }
    }

    #[test]
    fn test_windows_command_uvx() {
        let result = windows_command("uvx".to_string());
        if cfg!(windows) {
            assert_eq!(result, "uvx.cmd");
        } else {
            assert_eq!(result, "uvx");
        }
    }

    #[test]
    fn test_windows_command_pnpm() {
        let result = windows_command("pnpm".to_string());
        if cfg!(windows) {
            assert_eq!(result, "pnpm.cmd");
        } else {
            assert_eq!(result, "pnpm");
        }
    }

    #[test]
    fn test_windows_command_custom() {
        let result = windows_command("my-server".to_string());
        assert_eq!(result, "my-server");
    }

    #[test]
    fn test_default_config() {
        let config = McpClientConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.protocol_version, "2024-11-05");
        assert_eq!(config.client_name, "uni-mcp");
        assert_eq!(config.client_version, "0.1.0");
    }
}
