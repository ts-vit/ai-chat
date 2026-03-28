use std::sync::Arc;
use tauri::Manager;
use tauri::Emitter;
use uni_settings::JsonSettingsStore;

mod commands;

type SettingsState = Arc<JsonSettingsStore>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().expect("app_data_dir");
            std::fs::create_dir_all(&app_data_dir).ok();

            // Settings
            let settings_path = app_data_dir.join("settings.json");
            let store = Arc::new(JsonSettingsStore::new(settings_path));
            app.manage(store as SettingsState);

            // SSH Tunnel
            let ssh_tunnel_manager = Arc::new(uni_ssh::SshTunnelManager::new());
            app.manage(ssh_tunnel_manager.clone());
            {
                let mut rx = ssh_tunnel_manager.subscribe();
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    while let Ok(event) = rx.recv().await {
                        match event {
                            uni_ssh::SshEvent::Connected { host, port } => {
                                let _ = app_handle.emit("ssh-tunnel-connected", serde_json::json!({ "host": host, "port": port }));
                            }
                            uni_ssh::SshEvent::Disconnected => {
                                let _ = app_handle.emit("ssh-tunnel-disconnected", serde_json::json!({}));
                            }
                            uni_ssh::SshEvent::Reconnecting => {
                                let _ = app_handle.emit("ssh-tunnel-reconnecting", serde_json::json!({}));
                            }
                            uni_ssh::SshEvent::ReconnectAttempt { attempt, max_attempts } => {
                                let _ = app_handle.emit("ssh-tunnel-reconnect-attempt", serde_json::json!({ "attempt": attempt, "maxAttempts": max_attempts }));
                            }
                            uni_ssh::SshEvent::Reconnected { port } => {
                                let _ = app_handle.emit("ssh-tunnel-reconnected", serde_json::json!({ "port": port }));
                                let _ = app_handle.emit("proxy-settings-changed", ());
                            }
                            uni_ssh::SshEvent::ReconnectFailed => {
                                let _ = app_handle.emit("ssh-tunnel-reconnect-failed", serde_json::json!({}));
                            }
                            uni_ssh::SshEvent::HostKeyChanged { host, port } => {
                                let _ = app_handle.emit("ssh-host-key-changed", serde_json::json!({ "host": host, "port": port }));
                            }
                            uni_ssh::SshEvent::ProxySettingsChanged => {
                                let _ = app_handle.emit("proxy-settings-changed", ());
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core — settings
            commands::uni_settings::get_setting,
            commands::uni_settings::set_setting,
            commands::uni_settings::delete_setting,
            commands::uni_settings::get_all_settings,
            // SSH Tunnel
            commands::ssh_tunnel::ssh_tunnel_connect,
            commands::ssh_tunnel::ssh_tunnel_disconnect,
            commands::ssh_tunnel::ssh_tunnel_status,
            commands::ssh_tunnel::ssh_remove_known_host,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
