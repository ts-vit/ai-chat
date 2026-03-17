// команды для сохранения/загрузки API-ключа и настроек
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;

type Pool = sqlx::SqlitePool;

use crate::models::chat::{AppSettings, Model, ModelPricing};
use crate::services::http_client::build_http_client;

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
    store.set("sttProvider", serde_json::to_value(&settings.stt_provider).map_err(|e| e.to_string())?);
    store.set("sttLanguage", serde_json::to_value(&settings.stt_language).map_err(|e| e.to_string())?);
    store.set("openaiApiKey", serde_json::to_value(&settings.openai_api_key).map_err(|e| e.to_string())?);
    store.set("groqSttApiKey", serde_json::to_value(&settings.groq_stt_api_key).map_err(|e| e.to_string())?);
    store.set("ttsProvider", serde_json::to_value(&settings.tts_provider).map_err(|e| e.to_string())?);
    store.set("ttsVoice", serde_json::to_value(&settings.tts_voice).map_err(|e| e.to_string())?);
    store.set("ttsModel", serde_json::to_value(&settings.tts_model).map_err(|e| e.to_string())?);
    store.set("messageDensity", serde_json::to_value(&settings.message_density).map_err(|e| e.to_string())?);
    store.set("chatWidth", serde_json::to_value(&settings.chat_width).map_err(|e| e.to_string())?);
    store.set("showStatusBar", serde_json::to_value(settings.show_status_bar).map_err(|e| e.to_string())?);
    store.set("statusBarMetrics", serde_json::to_value(&settings.status_bar_metrics).map_err(|e| e.to_string())?);
    store.set("webSearchProvider", serde_json::to_value(&settings.web_search_provider).map_err(|e| e.to_string())?);
    store.set("tavilyApiKey", serde_json::to_value(&settings.tavily_api_key).map_err(|e| e.to_string())?);
    store.set("braveApiKey", serde_json::to_value(&settings.brave_api_key).map_err(|e| e.to_string())?);
    store.set("terminalFontSize", serde_json::to_value(&settings.terminal_font_size).map_err(|e| e.to_string())?);
    store.set("terminalShell", serde_json::to_value(&settings.terminal_shell).map_err(|e| e.to_string())?);
    store.set("proxyEnabled", serde_json::to_value(settings.proxy_enabled).map_err(|e| e.to_string())?);
    store.set("proxyType", serde_json::to_value(&settings.proxy_type).map_err(|e| e.to_string())?);
    store.set("proxyHost", serde_json::to_value(&settings.proxy_host).map_err(|e| e.to_string())?);
    store.set("proxyPort", serde_json::to_value(&settings.proxy_port).map_err(|e| e.to_string())?);
    store.set("proxyUsername", serde_json::to_value(&settings.proxy_username).map_err(|e| e.to_string())?);
    store.set("proxyPassword", serde_json::to_value(&settings.proxy_password).map_err(|e| e.to_string())?);
    store.set("sshHost", serde_json::to_value(&settings.ssh_host).map_err(|e| e.to_string())?);
    store.set("sshPort", serde_json::to_value(&settings.ssh_port).map_err(|e| e.to_string())?);
    store.set("sshUsername", serde_json::to_value(&settings.ssh_username).map_err(|e| e.to_string())?);
    store.set("sshAuthType", serde_json::to_value(&settings.ssh_auth_type).map_err(|e| e.to_string())?);
    store.set("sshPassword", serde_json::to_value(&settings.ssh_password).map_err(|e| e.to_string())?);
    store.set("sshKeyPath", serde_json::to_value(&settings.ssh_key_path).map_err(|e| e.to_string())?);
    store.set("sshAutoConnect", serde_json::to_value(settings.ssh_auto_connect).map_err(|e| e.to_string())?);
    store.set("routingEnabled", serde_json::to_value(settings.routing_enabled).map_err(|e| e.to_string())?);
    store.set("routingStrategy", serde_json::to_value(&settings.routing_strategy).map_err(|e| e.to_string())?);
    store.set("budgetPlanEnabled", serde_json::to_value(settings.budget_plan_enabled).map_err(|e| e.to_string())?);
    store.set("budgetPlanLimit", serde_json::to_value(&settings.budget_plan_limit).map_err(|e| e.to_string())?);
    store.set("budgetGlobalEnabled", serde_json::to_value(settings.budget_global_enabled).map_err(|e| e.to_string())?);
    store.set("budgetGlobalLimit", serde_json::to_value(&settings.budget_global_limit).map_err(|e| e.to_string())?);
    store.set("budgetGlobalPeriod", serde_json::to_value(&settings.budget_global_period).map_err(|e| e.to_string())?);
    store.set("modelCatalogLastSync", serde_json::to_value(&settings.model_catalog_last_sync).map_err(|e| e.to_string())?);
    store.set("telegramBotToken", serde_json::to_value(&settings.telegram_bot_token).map_err(|e| e.to_string())?);
    store.set("telegramEnabled", serde_json::to_value(settings.telegram_enabled).map_err(|e| e.to_string())?);
    store.set("telegramAutoStart", serde_json::to_value(settings.telegram_auto_start).map_err(|e| e.to_string())?);
    store.set("telegramModel", serde_json::to_value(&settings.telegram_model).map_err(|e| e.to_string())?);
    store.set("embeddingOpenaiKey", serde_json::to_value(&settings.embedding_openai_key).map_err(|e| e.to_string())?);
    store.set("embeddingGeminiKey", serde_json::to_value(&settings.embedding_gemini_key).map_err(|e| e.to_string())?);

    store.save().map_err(|e| e.to_string())?;

    let _ = app.emit("proxy-settings-changed", ());

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

    let status_bar_metrics: Vec<String> = store
        .get("statusBarMetrics")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| vec!["balance".into(), "context".into(), "tokens".into(), "cost".into()]);

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
        stt_provider: store.get("sttProvider").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        stt_language: store.get("sttLanguage").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        openai_api_key: store.get("openaiApiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        groq_stt_api_key: store.get("groqSttApiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        tts_provider: store.get("ttsProvider").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        tts_voice: store.get("ttsVoice").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        tts_model: store.get("ttsModel").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        message_density: store
            .get("messageDensity")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "standard".to_string()),
        chat_width: store
            .get("chatWidth")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "standard".to_string()),
        show_status_bar: store.get("showStatusBar").and_then(|v| v.as_bool()).unwrap_or(true),
        status_bar_metrics,
        web_search_provider: store.get("webSearchProvider").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        tavily_api_key: store.get("tavilyApiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        brave_api_key: store.get("braveApiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        terminal_font_size: store.get("terminalFontSize").and_then(|v| v.as_i64()).map(|v| v as i32),
        terminal_shell: store.get("terminalShell").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        proxy_enabled: store.get("proxyEnabled").and_then(|v| v.as_bool()).unwrap_or(false),
        proxy_type: store.get("proxyType").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        proxy_host: store.get("proxyHost").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        proxy_port: store.get("proxyPort").and_then(|v| v.as_u64()).map(|v| v as u16),
        proxy_username: store.get("proxyUsername").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        proxy_password: store.get("proxyPassword").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_host: store.get("sshHost").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_port: store.get("sshPort").and_then(|v| v.as_u64()).map(|v| v as u16),
        ssh_username: store.get("sshUsername").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_auth_type: store.get("sshAuthType").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_password: store.get("sshPassword").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_key_path: store.get("sshKeyPath").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        ssh_auto_connect: store.get("sshAutoConnect").and_then(|v| v.as_bool()).unwrap_or(false),
        routing_enabled: store.get("routingEnabled").and_then(|v| v.as_bool()).unwrap_or(false),
        routing_strategy: store.get("routingStrategy").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        budget_plan_enabled: store.get("budgetPlanEnabled").and_then(|v| v.as_bool()).unwrap_or(false),
        budget_plan_limit: store.get("budgetPlanLimit").and_then(|v| v.as_f64()),
        budget_global_enabled: store.get("budgetGlobalEnabled").and_then(|v| v.as_bool()).unwrap_or(false),
        budget_global_limit: store.get("budgetGlobalLimit").and_then(|v| v.as_f64()),
        budget_global_period: store.get("budgetGlobalPeriod").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        model_catalog_last_sync: store.get("modelCatalogLastSync").and_then(|v| v.as_i64()),
        telegram_bot_token: store.get("telegramBotToken").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        telegram_enabled: store.get("telegramEnabled").and_then(|v| v.as_bool()).unwrap_or(false),
        telegram_auto_start: store.get("telegramAutoStart").and_then(|v| v.as_bool()).unwrap_or(false),
        telegram_model: store.get("telegramModel").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        embedding_openai_key: store.get("embeddingOpenaiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
        embedding_gemini_key: store.get("embeddingGeminiKey").and_then(|v| v.as_str().map(String::from)).filter(|s| !s.is_empty()),
    };

    Ok(settings)
}

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

// Получить список моделей с OpenRouter
#[tauri::command]
pub async fn get_models(app: AppHandle) -> Result<serde_json::Value, String> {
    let client = build_http_client(&app, None).await?;

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

/// Test proxy connection by fetching external IP via httpbin.org.
#[tauri::command]
pub async fn test_proxy(
    proxy_type: String,
    proxy_host: String,
    proxy_port: u16,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
) -> Result<String, String> {
    let client = crate::services::http_client::build_http_client_from_params(
        &proxy_type,
        &proxy_host,
        proxy_port,
        proxy_username.as_deref(),
        proxy_password.as_deref(),
        Some(std::time::Duration::from_secs(10)),
    )?;
    let resp = client
        .get("https://httpbin.org/ip")
        .send()
        .await
        .map_err(|e| format!("{}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["origin"]
        .as_str()
        .unwrap_or("unknown")
        .to_string())
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