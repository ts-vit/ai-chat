// Сохранение и удаление файлов вложений
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

pub fn get_attachments_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("attachments");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Сохраняет файл вложения. Используется из send_message и из команды save_attachment.
pub fn save_attachment_file(
    app: &AppHandle,
    message_id: &str,
    file_name: &str,
    data: &[u8],
) -> Result<String, String> {
    let base_dir = get_attachments_dir(app)?;
    let sanitized_name = file_name.replace(['/', '\\'], "_");
    let mut path = base_dir.join(format!("{}_{}", message_id, sanitized_name));
    let mut suffix = 0u32;
    while path.exists() {
        suffix += 1;
        let stem = PathBuf::from(&sanitized_name);
        let ext = stem.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name_no_ext = stem.file_stem().and_then(|s| s.to_str()).unwrap_or(&sanitized_name);
        let new_name = if ext.is_empty() {
            format!("{}_{}_{}", message_id, name_no_ext, suffix)
        } else {
            format!("{}_{}_{}.{}", message_id, name_no_ext, suffix, ext)
        };
        path = base_dir.join(&new_name);
    }
    std::fs::write(&path, data).map_err(|e| e.to_string())?;
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let relative = path
        .strip_prefix(&app_data)
        .unwrap_or(&path);
    Ok(relative
        .to_str()
        .map(|s| s.replace('\\', "/"))
        .unwrap_or_else(|| format!("attachments/{}", path.file_name().unwrap_or_default().to_string_lossy())))
}

#[tauri::command]
pub async fn save_attachment(
    app: AppHandle,
    message_id: String,
    file_name: String,
    data: Vec<u8>,
) -> Result<String, String> {
    save_attachment_file(&app, &message_id, &file_name, &data)
}

/// Парсит content как JSON-массив блоков, для каждого с полем "path" удаляет файл из appDataDir.
/// Отсутствующие файлы игнорируются.
pub fn delete_attachments_for_message(app: &AppHandle, content: &str) {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(a) => a,
        Err(_) => return,
    };
    let base = match app.path().app_data_dir() {
        Ok(p) => p,
        Err(_) => return,
    };
    for block in arr {
        let path_str = match block.get("path").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => continue,
        };
        let full = base.join(path_str);
        let _ = std::fs::remove_file(&full);
    }
}
