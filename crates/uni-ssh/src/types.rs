use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTunnelStatus {
    pub connected: bool,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
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
