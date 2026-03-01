// команды для сохранения/загрузки API-ключа и настроек
use serde::Deserialize;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::models::chat::{AppSettings, Model, ModelPricing};

// Имя файла хранилища — будет создан автоматически в AppData
const STORE_NAME: &str = "settings.json";

// Сохранить настройки на диск
#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: AppSettings) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;

    store.set("api_key", serde_json::to_value(&settings.api_key).map_err(|e| e.to_string())?);
    store.set("management_key", serde_json::to_value(&settings.management_key).map_err(|e| e.to_string())?);
    store.set("model", serde_json::to_value(&settings.model).map_err(|e| e.to_string())?);
    store.set("temperature", serde_json::to_value(settings.temperature).map_err(|e| e.to_string())?);
    store.set("max_tokens", serde_json::to_value(settings.max_tokens).map_err(|e| e.to_string())?);
    store.set("font_size", serde_json::to_value(settings.font_size).map_err(|e| e.to_string())?);
    store.set("ollamaUrl", serde_json::to_value(&settings.ollama_url).map_err(|e| e.to_string())?);
    store.set("openrouterEnabledModels", serde_json::to_value(&settings.openrouter_enabled_models).map_err(|e| e.to_string())?);
    store.set("ollamaEnabledModels", serde_json::to_value(&settings.ollama_enabled_models).map_err(|e| e.to_string())?);
    store.set("customProviderEnabledModels", serde_json::to_value(&settings.custom_provider_enabled_models).map_err(|e| e.to_string())?);
    store.set("topP", serde_json::to_value(&settings.top_p).map_err(|e| e.to_string())?);
    store.set("topK", serde_json::to_value(&settings.top_k).map_err(|e| e.to_string())?);
    store.set("frequencyPenalty", serde_json::to_value(&settings.frequency_penalty).map_err(|e| e.to_string())?);
    store.set("presencePenalty", serde_json::to_value(&settings.presence_penalty).map_err(|e| e.to_string())?);
    store.set("language", serde_json::to_value(&settings.language).map_err(|e| e.to_string())?);
    store.set("sendByEnter", serde_json::to_value(&settings.send_by_enter).map_err(|e| e.to_string())?);

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

// Загрузить настройки с диска (если нет — вернёт дефолтные)
#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;

    let openrouter_enabled_models: Vec<String> = store
        .get("openrouterEnabledModels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let ollama_enabled_models: Vec<String> = store
        .get("ollamaEnabledModels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let custom_provider_enabled_models: std::collections::HashMap<String, Vec<String>> = store
        .get("customProviderEnabledModels")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let settings = AppSettings {
        api_key: store.get("api_key")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        management_key: store.get("management_key")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        model: store.get("model")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string()),
        temperature: store.get("temperature")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32)
            .unwrap_or(0.7),
        max_tokens: store.get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(4096),
        font_size: store.get("font_size")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(14),
        ollama_url: store.get("ollamaUrl")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "http://localhost:11434/v1".to_string()),
        openrouter_enabled_models,
        ollama_enabled_models,
        custom_provider_enabled_models,
        top_p: store.get("topP").and_then(|v| v.as_f64()).map(|v| v as f32),
        top_k: store.get("topK").and_then(|v| v.as_u64()).map(|v| v as u32),
        frequency_penalty: store.get("frequencyPenalty").and_then(|v| v.as_f64()).map(|v| v as f32),
        presence_penalty: store.get("presencePenalty").and_then(|v| v.as_f64()).map(|v| v as f32),
        language: store.get("language")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default(),
        send_by_enter: store.get("sendByEnter").and_then(|v| v.as_bool()).unwrap_or(true),
    };

    Ok(settings)
}

// Получить кредиты OpenRouter (Management key)
#[tauri::command]
pub async fn get_credits(management_key: String) -> Result<serde_json::Value, String> {
    if management_key.is_empty() {
        return Err("Management key is empty".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
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
pub async fn get_balance(api_key: String) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
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

// Получить список моделей с OpenRouter
#[tauri::command]
pub async fn get_models() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();

    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| e.to_string())?;

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
pub async fn get_ollama_models(base_url: String) -> Result<Vec<Model>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    fetch_ollama_models(&client, &base_url).await
}

/// Загрузить список моделей кастомного провайдера (GET {base_url}/v1/models с api_key).
#[tauri::command]
pub async fn fetch_custom_provider_models(base_url: String, api_key: String) -> Result<Vec<Model>, String> {
    let base_url = base_url.trim_end_matches('/');
    let url = format!("{}/v1/models", base_url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
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