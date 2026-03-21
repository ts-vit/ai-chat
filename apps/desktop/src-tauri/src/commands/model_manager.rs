use crate::services::model_manager;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::{Emitter, Manager};

fn get_model_dirs(app: &AppHandle) -> Result<(PathBuf, PathBuf), String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
    let app_model_dir = app_data_dir.join("models").join("multilingual-e5-small");
    let mut bundled_model_dir = resource_dir.join("models").join("multilingual-e5-small");

    // Dev fallback
    if !bundled_model_dir.join("model_int8.onnx").exists() {
        if let Ok(cwd) = std::env::current_dir() {
            let dev_dir = cwd.join("resources").join("models").join("multilingual-e5-small");
            if dev_dir.join("model_int8.onnx").exists() {
                bundled_model_dir = dev_dir;
            }
        }
    }

    Ok((app_model_dir, bundled_model_dir))
}

#[tauri::command]
pub async fn get_model_status(app: AppHandle) -> Result<model_manager::ModelStatus, String> {
    let (app_model_dir, bundled_model_dir) = get_model_dirs(&app)?;
    let mut status = model_manager::get_model_status(&app_model_dir, &bundled_model_dir);

    match model_manager::fetch_manifest(&app).await {
        Ok(manifest) => {
            let total_size: u64 = manifest.files.iter().map(|f| f.size).sum();
            status.latest_version = Some(manifest.version.clone());
            status.model_size_mb = Some(total_size as f64 / 1_048_576.0);
            status.update_available = match &status.current_version {
                Some(current) => current != &manifest.version,
                None => status.source == "bundled" || status.source == "none",
            };
        }
        Err(e) => {
            log::warn!("Failed to fetch model manifest: {}", e);
        }
    }

    Ok(status)
}

#[tauri::command]
pub async fn download_embedding_model(app: AppHandle) -> Result<(), String> {
    let (app_model_dir, _) = get_model_dirs(&app)?;

    let manifest = model_manager::fetch_manifest(&app)
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    model_manager::download_model(&manifest, &app_model_dir, &app)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    let _ = app.emit("model-download-complete", ());

    Ok(())
}

#[tauri::command]
pub async fn delete_downloaded_model(app: AppHandle) -> Result<(), String> {
    let (app_model_dir, _) = get_model_dirs(&app)?;
    if app_model_dir.exists() {
        std::fs::remove_dir_all(&app_model_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}
