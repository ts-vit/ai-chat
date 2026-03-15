use std::sync::Mutex;

use tauri::AppHandle;
use tauri::State;

use crate::services::http_client::get_active_proxy_url;
use crate::services::terminal::TerminalManager;

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
    let id = uuid::Uuid::new_v4().to_string();
    let proxy_url = get_active_proxy_url(&app_handle).await;
    let mut mgr = manager.lock().map_err(|e| e.to_string())?;
    mgr.create_session(id.clone(), cols as u16, rows as u16, app_handle, shell, proxy_url)?;
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
