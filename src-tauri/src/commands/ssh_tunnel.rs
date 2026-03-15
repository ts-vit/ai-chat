use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::services::ssh_tunnel::{SshTunnelManager, SshTunnelStatus};

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
    manager
        .connect(app.clone(), host, port, username, auth_type, password, private_key)
        .await
}

#[tauri::command]
pub async fn ssh_tunnel_disconnect(app: AppHandle) -> Result<(), String> {
    let manager = app.state::<Arc<SshTunnelManager>>();
    manager.disconnect(Some(&app)).await
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
    let prefix = format!("{}:{} ", host, port);
    if path.exists() {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let filtered: String = content
            .lines()
            .filter(|l| !l.starts_with(&prefix))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(&path, filtered).map_err(|e| e.to_string())?;
    }
    Ok(())
}
