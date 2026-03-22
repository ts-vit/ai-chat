use std::sync::Arc;
use tauri::{AppHandle, Manager};

use uni_ssh::{SshConfig, SshTunnelManager, SshTunnelStatus};

#[tauri::command]
pub async fn ssh_tunnel_connect(
    app: AppHandle,
    host: String,
    port: u16,
    username: String,
    auth_type: String,
    password: Option<String>,
    private_key: Option<String>,
) -> Result<u16, String> {
    let manager = app.state::<Arc<SshTunnelManager>>();
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let config = SshConfig {
        host,
        port,
        username,
        auth_type,
        password,
        private_key,
        known_hosts_path: app_data_dir.join("ssh_known_hosts"),
    };
    manager.connect(config).await
}

#[tauri::command]
pub async fn ssh_tunnel_disconnect(app: AppHandle) -> Result<(), String> {
    let manager = app.state::<Arc<SshTunnelManager>>();
    manager.disconnect().await
}

#[tauri::command]
pub async fn ssh_tunnel_status(app: AppHandle) -> Result<SshTunnelStatus, String> {
    let manager = app.state::<Arc<SshTunnelManager>>();
    Ok(manager.get_status().await)
}

#[tauri::command]
pub async fn ssh_remove_known_host(app: AppHandle, host: String, port: u16) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = app_data_dir.join("ssh_known_hosts");
    uni_ssh::remove_known_host(&path, &host, port)
}
