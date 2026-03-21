use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 Request (Rust → Python via stdin)
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 Response (Python → Rust via stdout)
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Notification (Python → Rust, no id — for progress)
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

/// Parsed message from Python stdout (can be Response or Notification)
#[derive(Debug, Clone)]
pub enum PythonMessage {
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

/// Parse a line from stdout as a JSON-RPC message.
///
/// Lines that are not valid JSON or not JSON-RPC are returned as None
/// (they may be stray print() output — logged but ignored).
pub fn parse_message(line: &str) -> Option<PythonMessage> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    let value: serde_json::Value = serde_json::from_str(line).ok()?;

    // Must have "jsonrpc": "2.0"
    if value.get("jsonrpc")?.as_str()? != "2.0" {
        return None;
    }

    // If has "id" — it's a Response; otherwise a Notification
    if value.get("id").is_some() {
        let resp: JsonRpcResponse = serde_json::from_value(value).ok()?;
        Some(PythonMessage::Response(resp))
    } else if value.get("method").is_some() {
        let notif: JsonRpcNotification = serde_json::from_value(value).ok()?;
        Some(PythonMessage::Notification(notif))
    } else {
        None
    }
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    // Custom application error codes
    pub const TIMEOUT: i32 = -32000;
    pub const CANCELLED: i32 = -32001;
    pub const DEPENDENCY_ERROR: i32 = -32002;
    pub const SANDBOX_ERROR: i32 = -32003;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialize() {
        let req = JsonRpcRequest::new(1, "convert", serde_json::json!({"path": "/tmp/f.pdf"}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"convert\""));
    }

    #[test]
    fn test_parse_response() {
        let line = r##"{"jsonrpc":"2.0","id":1,"result":{"markdown":"# Hello"}}"##;
        let msg = parse_message(line).unwrap();
        match msg {
            PythonMessage::Response(r) => {
                assert_eq!(r.id, Some(1));
                assert!(r.result.is_some());
                assert!(r.error.is_none());
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_parse_error_response() {
        let line = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"Invalid params"}}"#;
        let msg = parse_message(line).unwrap();
        match msg {
            PythonMessage::Response(r) => {
                assert!(r.error.is_some());
                assert_eq!(r.error.unwrap().code, -32602);
            }
            _ => panic!("Expected Response"),
        }
    }

    #[test]
    fn test_parse_notification() {
        let line = r#"{"jsonrpc":"2.0","method":"progress","params":{"percent":50,"message":"Processing..."}}"#;
        let msg = parse_message(line).unwrap();
        match msg {
            PythonMessage::Notification(n) => {
                assert_eq!(n.method, "progress");
                let params = n.params.unwrap();
                assert_eq!(params["percent"], 50);
            }
            _ => panic!("Expected Notification"),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_message("not json at all").is_none());
    }

    #[test]
    fn test_parse_non_jsonrpc() {
        assert!(parse_message(r#"{"hello":"world"}"#).is_none());
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_message("").is_none());
        assert!(parse_message("   ").is_none());
    }

    #[test]
    fn test_parse_stray_print() {
        // Python script did print("debug info") — should be ignored
        assert!(parse_message("debug info").is_none());
    }
}
