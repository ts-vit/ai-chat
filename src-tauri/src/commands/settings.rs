// команды для сохранения/загрузки API-ключа и настроек
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::models::chat::AppSettings;

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

    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

// Загрузить настройки с диска (если нет — вернёт дефолтные)
#[tauri::command]
pub async fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;

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