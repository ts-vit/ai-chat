// Команды управления Ollama через HTTP API (check/list/pull/delete)
use std::time::Duration;

use futures_util::StreamExt;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;

use crate::commands::settings::fetch_ollama_models;

const STORE_NAME: &str = "settings.json";
const OLLAMA_CHECK_TIMEOUT_SECS: u64 = 3;
const OLLAMA_DELETE_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaLocalModel {
    pub name: String,
    pub size: String,
}

/// Возвращает URL Ollama из store (с /v1).
fn get_ollama_url_from_store(app: &AppHandle) -> String {
    let store = match app.store(STORE_NAME) {
        Ok(s) => s,
        Err(_) => return "http://localhost:11434/v1".to_string(),
    };
    store
        .get("ollamaUrl")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:11434/v1".to_string())
}

/// Базовый URL Ollama без суффикса /v1 (для /api/pull, /api/delete и т.д.).
fn get_ollama_base_url(app: &AppHandle) -> String {
    let url = get_ollama_url_from_store(app);
    let base = if url.ends_with("/v1") {
        url.trim_end_matches("/v1").trim_end_matches('/')
    } else {
        url.trim_end_matches('/')
    };
    if base.is_empty() {
        "http://localhost:11434".to_string()
    } else {
        base.to_string()
    }
}

/// Проверяет доступность Ollama по HTTP (GET базовый URL, таймаут 3 с).
#[tauri::command]
pub async fn check_ollama_status(app: AppHandle) -> Result<bool, String> {
    let base = get_ollama_base_url(&app);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(OLLAMA_CHECK_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    match client.get(&base).send().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Возвращает список локальных моделей Ollama через HTTP API.
#[tauri::command]
pub async fn get_local_ollama_models(app: AppHandle) -> Result<Vec<OllamaLocalModel>, String> {
    let url = get_ollama_url_from_store(&app);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let models = fetch_ollama_models(&client, &url).await?;
    Ok(models
        .into_iter()
        .map(|m| OllamaLocalModel {
            name: m.id,
            size: String::new(),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaPullProgressPayload {
    pub status: String,
    pub completed: Option<u64>,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaPullDonePayload {
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaPullErrorPayload {
    pub message: String,
}

/// Запускает скачивание модели через HTTP API (POST /api/pull) и сразу возвращает Ok(()).
/// Прогресс и результат приходят через события: ollama-pull-progress, ollama-pull-done, ollama-pull-error.
#[tauri::command]
pub async fn pull_ollama_model(app: AppHandle, model_name: String) -> Result<(), String> {
    let base = get_ollama_base_url(&app);
    let pull_url = format!("{}/api/pull", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post(&pull_url)
        .json(&serde_json::json!({ "name": &model_name, "stream": true }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let msg = response.text().await.unwrap_or_else(|_| status.to_string());
        return Err(format!("Ollama pull: {}", msg));
    }

    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(b) => b,
                Err(e) => {
                    let _ = app_clone.emit(
                        "ollama-pull-error",
                        OllamaPullErrorPayload {
                            message: e.to_string(),
                        },
                    );
                    return;
                }
            };
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let status = parsed
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();

                if status == "success" {
                    let _ = app_clone.emit(
                        "ollama-pull-done",
                        OllamaPullDonePayload {
                            model: model_name.clone(),
                        },
                    );
                    return;
                }
                if status.to_lowercase().contains("error") {
                    let msg = parsed
                        .get("error")
                        .and_then(|e| e.as_str())
                        .unwrap_or(&status)
                        .to_string();
                    let _ = app_clone.emit(
                        "ollama-pull-error",
                        OllamaPullErrorPayload { message: msg },
                    );
                    return;
                }

                let completed = parsed.get("completed").and_then(|c| c.as_u64());
                let total = parsed.get("total").and_then(|t| t.as_u64());
                if completed.is_some() && total.is_some() && total.unwrap_or(0) > 0 {
                    let _ = app_clone.emit(
                        "ollama-pull-progress",
                        OllamaPullProgressPayload {
                            status,
                            completed,
                            total,
                        },
                    );
                }
            }
        }

        let _ = app_clone.emit(
            "ollama-pull-error",
            OllamaPullErrorPayload {
                message: "Стрим завершился без success".to_string(),
            },
        );
    });

    Ok(())
}

/// Удаляет локальную модель Ollama через HTTP API (DELETE /api/delete).
#[tauri::command]
pub async fn delete_ollama_model(app: AppHandle, model_name: String) -> Result<(), String> {
    let base = get_ollama_base_url(&app);
    let delete_url = format!("{}/api/delete", base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(OLLAMA_DELETE_TIMEOUT_SECS))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .delete(&delete_url)
        .json(&serde_json::json!({ "name": &model_name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let msg = response.text().await.unwrap_or_else(|_| status.to_string());
        Err(format!("Ollama delete: {}", msg))
    }
}
