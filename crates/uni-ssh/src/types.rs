use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelStatus {
    pub connected: bool,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
}

/// Local port forwarding config (analogous to `ssh -L localPort:remoteHost:remotePort`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortForwardConfig {
    /// Local port to bind (0 = auto-assign)
    pub local_port: u16,
    /// Remote host to connect to through the SSH tunnel (usually "127.0.0.1" for server-local services)
    pub remote_host: String,
    /// Remote port to connect to
    pub remote_port: u16,
}

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub known_hosts_path: PathBuf,
    /// If set, use local port forwarding instead of SOCKS5 proxy
    pub port_forward: Option<PortForwardConfig>,
}

/// Events emitted by the SSH tunnel manager.
#[derive(Debug, Clone)]
pub enum SshEvent {
    Connected { host: String, port: u16 },
    Disconnected,
    Reconnecting,
    ReconnectAttempt { attempt: u32, max_attempts: u32 },
    Reconnected { port: u16 },
    ReconnectFailed,
    HostKeyChanged { host: String, port: u16 },
    ProxySettingsChanged,
}
