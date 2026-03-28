use std::sync::Arc;
use tauri::State;
use uni_settings::{JsonSettingsStore, Setting, SettingsStore};

type SettingsState = Arc<JsonSettingsStore>;

#[tauri::command]
pub async fn get_setting(
    store: State<'_, SettingsState>,
    key: String,
) -> Result<Option<String>, String> {
    store.get(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(
    store: State<'_, SettingsState>,
    key: String,
    value: String,
) -> Result<(), String> {
    store.set(&key, &value).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_setting(
    store: State<'_, SettingsState>,
    key: String,
) -> Result<(), String> {
    store.delete(&key).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_all_settings(
    store: State<'_, SettingsState>,
    prefix: String,
) -> Result<Vec<Setting>, String> {
    store.list(&prefix).await.map_err(|e| e.to_string())
}
