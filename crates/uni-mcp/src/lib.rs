//! uni-mcp — MCP (Model Context Protocol) client for UNI Framework
//!
//! Provides JSON-RPC 2.0 client over stdio for connecting to MCP servers,
//! and a manager for handling multiple server connections.

mod client;
mod error;
mod manager;
mod types;

pub use client::{McpClient, McpClientConfig};
pub use error::McpError;
pub use manager::McpManager;
pub use types::{McpConnectionInfo, McpContent, McpTool, McpToolInfo, McpToolResult};
