use std::collections::HashMap;
use std::path::PathBuf;

/// Events emitted by terminal sessions.
/// Application converts these to Tauri events, etc.
#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Data { session_id: String, data: String },
    Exit { session_id: String, code: i32 },
}

/// Configuration for creating a terminal session.
pub struct TerminalConfig {
    pub cols: u16,
    pub rows: u16,
    /// Shell to use. None = auto-detect.
    pub shell: Option<String>,
    /// Working directory. None = home dir.
    pub cwd: Option<PathBuf>,
    /// Extra environment variables (e.g. proxy settings).
    pub env: HashMap<String, String>,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            shell: None,
            cwd: None,
            env: HashMap::new(),
        }
    }
}
