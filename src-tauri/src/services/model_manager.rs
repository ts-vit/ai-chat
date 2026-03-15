use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Emitter;

use crate::services::http_client;

const MANIFEST_URL: &str = "https://github.com/ts-vit/ai-chat/releases/latest/download/model-manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifest {
    pub model_name: String,
    pub version: String,
    pub files: Vec<ModelFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFile {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalVersion {
    pub version: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub installed: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub model_size_mb: Option<f64>,
    pub source: String,
}

#[derive(Clone, Serialize)]
pub struct DownloadProgress {
    pub file_name: String,
    pub downloaded: u64,
    pub total: u64,
    pub percent: f64,
}

pub async fn fetch_manifest(app: &AppHandle) -> Result<ModelManifest, String> {
    let client = http_client::build_http_client(
        app,
        Some(std::time::Duration::from_secs(15)),
    ).await?;

    let response = client
        .get(MANIFEST_URL)
        .header("User-Agent", "ai-chat")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status(), MANIFEST_URL));
    }

    response
        .json::<ModelManifest>()
        .await
        .map_err(|e| format!("Failed to parse manifest: {}", e))
}

pub fn read_local_version(model_dir: &PathBuf) -> Option<LocalVersion> {
    let version_file = model_dir.join("version.json");
    if version_file.exists() {
        let content = std::fs::read_to_string(&version_file).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

pub fn write_local_version(model_dir: &PathBuf, version: &str) -> Result<(), String> {
    std::fs::create_dir_all(model_dir).map_err(|e| e.to_string())?;
    let local = LocalVersion {
        version: version.to_string(),
        installed_at: chrono::Utc::now().to_rfc3339(),
    };
    let content = serde_json::to_string_pretty(&local).map_err(|e| e.to_string())?;
    std::fs::write(model_dir.join("version.json"), content).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_model_status(app_model_dir: &PathBuf, bundled_model_dir: &PathBuf) -> ModelStatus {
    let app_model_exists = app_model_dir.join("model_int8.onnx").exists();
    let bundled_exists = bundled_model_dir.join("model_int8.onnx").exists();

    let (installed, source, current_version) = if app_model_exists {
        let ver = read_local_version(app_model_dir).map(|v| v.version);
        (true, "downloaded".to_string(), ver)
    } else if bundled_exists {
        (true, "bundled".to_string(), None)
    } else {
        (false, "none".to_string(), None)
    };

    ModelStatus {
        installed,
        current_version,
        latest_version: None,
        update_available: false,
        model_size_mb: None,
        source,
    }
}

pub async fn download_file(
    url: &str,
    dest: &PathBuf,
    expected_sha256: &str,
    expected_size: u64,
    file_name: &str,
    app_handle: &AppHandle,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let client = http_client::build_http_client(
        app_handle,
        Some(std::time::Duration::from_secs(600)),
    ).await?;

    let response = client
        .get(url)
        .header("User-Agent", "ai-chat")
        .send()
        .await
        .map_err(|e| format!("Download request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status(), url));
    }

    let total = response.content_length().unwrap_or(expected_size);

    let temp_path = dest.with_extension("tmp");
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut hasher = Sha256::new();
    let mut last_emit = std::time::Instant::now();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {}", e))?;
        downloaded += chunk.len() as u64;

        if last_emit.elapsed().as_millis() > 200 || downloaded == total {
            let progress = DownloadProgress {
                file_name: file_name.to_string(),
                downloaded,
                total,
                percent: if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 },
            };
            let _ = app_handle.emit("model-download-progress", &progress);
            last_emit = std::time::Instant::now();
        }
    }
    file.flush().await.map_err(|e| format!("Flush error: {}", e))?;
    drop(file);

    let hash = format!("{:x}", hasher.finalize());
    if hash != expected_sha256 {
        let _ = tokio::fs::remove_file(&temp_path).await;
        return Err(format!(
            "SHA256 mismatch for {}: expected {}, got {}",
            file_name, expected_sha256, hash
        ));
    }

    tokio::fs::rename(&temp_path, dest)
        .await
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;

    Ok(())
}

pub async fn download_model(
    manifest: &ModelManifest,
    model_dir: &PathBuf,
    app_handle: &AppHandle,
) -> Result<(), String> {
    std::fs::create_dir_all(model_dir).map_err(|e| e.to_string())?;

    for file_info in &manifest.files {
        let dest = model_dir.join(&file_info.name);
        download_file(
            &file_info.url,
            &dest,
            &file_info.sha256,
            file_info.size,
            &file_info.name,
            app_handle,
        )
        .await?;
    }

    write_local_version(model_dir, &manifest.version)?;

    Ok(())
}
