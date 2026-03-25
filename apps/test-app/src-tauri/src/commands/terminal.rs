use std::collections::HashMap;
use std::sync::Mutex;

use tauri::AppHandle;
use tauri::State;

use uni_terminal::{TerminalConfig, TerminalManager};

use crate::services::http_client::get_active_proxy_url;

#[tauri::command]
pub async fn get_current_proxy_url(app: AppHandle) -> Result<Option<String>, String> {
    Ok(get_active_proxy_url(&app).await)
}

#[tauri::command]
pub async fn terminal_create(
    manager: State<'_, Mutex<TerminalManager>>,
    app_handle: AppHandle,
    cols: u32,
    rows: u32,
    shell: Option<String>,
) -> Result<String, String> {
    let id = uni_common::generate_id();
    let proxy_url = get_active_proxy_url(&app_handle).await;

    let mut env = HashMap::new();
    if let Some(proxy) = proxy_url {
        for key in &[
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ] {
            env.insert(key.to_string(), proxy.clone());
        }
    }

    let config = TerminalConfig {
        cols: cols as u16,
        rows: rows as u16,
        shell,
        cwd: None,
        env,
    };

    let mut mgr = manager.lock().map_err(|e| e.to_string())?;
    mgr.create_session(id.clone(), config)?;
    Ok(id)
}

#[tauri::command]
pub async fn terminal_write(
    manager: State<'_, Mutex<TerminalManager>>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    let mut mgr = manager.lock().map_err(|e| e.to_string())?;
    mgr.write_to_session(&session_id, &data)
}

#[tauri::command]
pub async fn terminal_resize(
    manager: State<'_, Mutex<TerminalManager>>,
    session_id: String,
    cols: u32,
    rows: u32,
) -> Result<(), String> {
    let mut mgr = manager.lock().map_err(|e| e.to_string())?;
    mgr.resize_session(&session_id, cols as u16, rows as u16)
}

#[tauri::command]
pub async fn terminal_kill(
    manager: State<'_, Mutex<TerminalManager>>,
    session_id: String,
) -> Result<(), String> {
    let mut mgr = manager.lock().map_err(|e| e.to_string())?;
    mgr.kill_session(&session_id)
}
