use std::fmt;

#[derive(Debug)]
pub enum McpError {
    /// Failed to spawn server process
    Spawn(String),
    /// JSON-RPC protocol error
    Rpc { code: i64, message: String },
    /// Communication error (stdin/stdout)
    Io(String),
    /// Request timeout
    Timeout { method: String, timeout_secs: u64 },
    /// Server not found in manager
    ServerNotFound(String),
    /// Serialization error
    Serde(String),
    /// Other error
    Other(String),
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            McpError::Spawn(msg) => write!(f, "Failed to spawn MCP server: {}", msg),
            McpError::Rpc { code, message } => write!(f, "MCP RPC error {}: {}", code, message),
            McpError::Io(msg) => write!(f, "MCP I/O error: {}", msg),
            McpError::Timeout {
                method,
                timeout_secs,
            } => {
                write!(f, "MCP request timeout: {} ({}s)", method, timeout_secs)
            }
            McpError::ServerNotFound(id) => write!(f, "MCP server not found: {}", id),
            McpError::Serde(msg) => write!(f, "MCP serialization error: {}", msg),
            McpError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for McpError {}

impl From<McpError> for String {
    fn from(e: McpError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = McpError::Spawn("command not found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("spawn"));
        assert!(msg.contains("command not found"));
    }

    #[test]
    fn test_error_rpc_display() {
        let err = McpError::Rpc {
            code: -32600,
            message: "Invalid Request".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("-32600"));
        assert!(msg.contains("Invalid Request"));
    }

    #[test]
    fn test_error_timeout_display() {
        let err = McpError::Timeout {
            method: "tools/list".to_string(),
            timeout_secs: 30,
        };
        let msg = err.to_string();
        assert!(msg.contains("tools/list"));
        assert!(msg.contains("30"));
    }

    #[test]
    fn test_error_to_string() {
        let err = McpError::ServerNotFound("test-id".to_string());
        let s: String = err.into();
        assert!(s.contains("test-id"));
    }

    #[test]
    fn test_error_io() {
        let err = McpError::Io("broken pipe".to_string());
        assert!(err.to_string().contains("broken pipe"));
    }

    #[test]
    fn test_error_serde() {
        let err = McpError::Serde("invalid json".to_string());
        assert!(err.to_string().contains("invalid json"));
    }

    #[test]
    fn test_error_other() {
        let err = McpError::Other("something went wrong".to_string());
        assert_eq!(err.to_string(), "something went wrong");
    }
}
