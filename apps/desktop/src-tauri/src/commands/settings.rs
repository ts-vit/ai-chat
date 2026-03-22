// команды для настроек, моделей и провайдеров
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::AppHandle;

type Pool = sqlx::SqlitePool;

use uni_llm::{Model, ModelPricing};
use crate::services::http_client::build_http_client;

// Получить кредиты OpenRouter (Management key)
#[tauri::command]
pub async fn get_credits(app: AppHandle, management_key: String) -> Result<serde_json::Value, String> {
    if management_key.is_empty() {
        return Err("Management key is empty".to_string());
    }
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(10))).await?;
    let response = client
        .get("https://openrouter.ai/api/v1/credits")
        .header("Authorization", format!("Bearer {}", management_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let err_msg = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        return Err(format!("{}: {}", status, err_msg));
    }
    Ok(json)
}

// Получить баланс ключа OpenRouter
#[tauri::command]
pub async fn get_balance(app: AppHandle, api_key: String) -> Result<serde_json::Value, String> {
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(10))).await?;
    let response = client
        .get("https://openrouter.ai/api/v1/auth/key")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let err_msg = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown API error");
        return Err(format!("{}: {}", status, err_msg));
    }
    Ok(json)
}

/// OpenAI-совместимый ответ: { "data": [ { "id": "..." } ] }
#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Option<Vec<OpenAIModelItem>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelItem {
    id: String,
}

/// Нативный Ollama: { "models": [ { "name": "..." } ] }
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Option<Vec<OllamaModelItem>>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelItem {
    name: String,
}

fn to_model(id: String) -> Model {
    Model {
        id: id.clone(),
        name: id,
        context_length: 0,
        pricing: ModelPricing {
            prompt: "0".to_string(),
            completion: "0".to_string(),
        },
        supports_vision: false,
        supports_tool_use: true,
    }
}

/// GET {base_url}/models (OpenAI-формат), при 404 или пустом data — GET /api/tags (нативный Ollama).
pub async fn fetch_ollama_models(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<Model>, String> {
    let base_url = base_url.trim_end_matches('/');
    let models_url = format!("{}/models", base_url);

    let response = client.get(&models_url).send().await.map_err(|e| e.to_string())?;
    let status = response.status();

    if status.is_success() {
        let json: OpenAIModelsResponse = response.json().await.map_err(|e| e.to_string())?;
        if let Some(data) = json.data {
            if !data.is_empty() {
                let list: Vec<Model> = data
                    .into_iter()
                    .map(|m| to_model(m.id))
                    .collect();
                return Ok(list);
            }
        }
    }

    let tags_url = base_url.replace("/v1", "/api/tags");
    let tags_response = client
        .get(&tags_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !tags_response.status().is_success() {
        return Err(format!(
            "Ollama: ни OpenAI /models ({}), ни /api/tags не вернули список моделей",
            status
        ));
    }

    let tags_json: OllamaTagsResponse = tags_response
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let list: Vec<Model> = tags_json
        .models
        .unwrap_or_default()
        .into_iter()
        .map(|m| to_model(m.name))
        .collect();

    Ok(list)
}

/// Список моделей Ollama (HTTP API).
#[tauri::command]
pub async fn get_ollama_models(app: AppHandle, base_url: String) -> Result<Vec<Model>, String> {
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(15))).await?;
    fetch_ollama_models(&client, &base_url).await
}

/// Загрузить список моделей кастомного провайдера (GET {base_url}/v1/models с api_key).
#[tauri::command]
pub async fn fetch_custom_provider_models(app: AppHandle, base_url: String, api_key: String) -> Result<Vec<Model>, String> {
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{}/v1/models", base_url);
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(15))).await?;
    let mut request = client.get(&url);
    if !api_key.is_empty() {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("{}: {}", status, body));
    }
    let json: OpenAIModelsResponse = response.json().await.map_err(|e| e.to_string())?;
    let list = json
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|m| to_model(m.id))
        .collect();
    Ok(list)
}

/// Validate OpenRouter API key by hitting /api/v1/models.
#[tauri::command]
pub async fn validate_openrouter_key(app: AppHandle, api_key: String) -> Result<bool, String> {
    let client = build_http_client(&app, Some(std::time::Duration::from_secs(5))).await?;
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(&api_key)
        .send()
        .await
        .map_err(|e| format!("Не удалось подключиться к OpenRouter: {}", e))?;
    Ok(resp.status().is_success())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeSettingRow {
    pub mode: String,
    pub enabled: bool,
    pub sort_order: i64,
    pub config: String,
}

#[tauri::command]
pub async fn get_mode_settings(pool: tauri::State<'_, Pool>) -> Result<Vec<ModeSettingRow>, String> {
    let rows = sqlx::query("SELECT mode, enabled, sort_order, config FROM mode_settings ORDER BY sort_order")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    let settings = rows
        .into_iter()
        .map(|row| ModeSettingRow {
            mode: row.get("mode"),
            enabled: row.try_get::<i64, _>("enabled").unwrap_or(1) != 0,
            sort_order: row.get("sort_order"),
            config: row.get("config"),
        })
        .collect();
    Ok(settings)
}

#[tauri::command]
pub async fn update_mode_settings(
    pool: tauri::State<'_, Pool>,
    mode: String,
    enabled: bool,
    sort_order: i64,
    config: String,
) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO mode_settings (mode, enabled, sort_order, config) VALUES (?, ?, ?, ?)")
        .bind(&mode)
        .bind(enabled as i64)
        .bind(sort_order)
        .bind(&config)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}