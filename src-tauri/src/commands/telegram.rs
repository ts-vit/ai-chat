// Telegram bot Tauri commands — start/stop/status/auth
use std::sync::Arc;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

use crate::services::telegram_bot::{TelegramBotManager, get_bot_info};

type Pool = sqlx::SqlitePool;
const STORE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramStatus {
    pub running: bool,
    pub bot_username: Option<String>,
    pub authorized_user: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramBotInfo {
    pub username: String,
    pub first_name: String,
}

#[tauri::command]
pub async fn start_telegram_bot(
    app: AppHandle,
    pool: State<'_, Pool>,
    telegram_state: State<'_, Arc<TelegramBotManager>>,
) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let token = store.get("telegramBotToken")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    if token.is_empty() {
        return Err("Telegram bot token is not configured".to_string());
    }

    telegram_state.start(app, pool.inner().clone(), token).await?;
    Ok(())
}

#[tauri::command]
pub async fn stop_telegram_bot(
    app: AppHandle,
    telegram_state: State<'_, Arc<TelegramBotManager>>,
) -> Result<(), String> {
    telegram_state.stop(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn get_telegram_status(
    app: AppHandle,
    telegram_state: State<'_, Arc<TelegramBotManager>>,
) -> Result<TelegramStatus, String> {
    let running = telegram_state.is_running().await;
    let bot_username = telegram_state.bot_username().await;

    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let authorized_user = store.get("telegramUserName")
        .and_then(|v| v.as_str().map(String::from));

    Ok(TelegramStatus {
        running,
        bot_username,
        authorized_user,
    })
}

#[tauri::command]
pub async fn authorize_telegram_user(
    app: AppHandle,
    user_id: i64,
    first_name: String,
    last_name: Option<String>,
    username: Option<String>,
) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    store.set("telegramUserId", serde_json::json!(user_id));

    let display_name = if let Some(ref uname) = username {
        format!("{} (@{})", first_name, uname)
    } else if let Some(ref lname) = last_name {
        format!("{} {}", first_name, lname)
    } else {
        first_name
    };

    store.set("telegramUserName", serde_json::json!(display_name));
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn revoke_telegram_user(
    app: AppHandle,
) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    store.delete("telegramUserId");
    store.delete("telegramUserName");
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn validate_telegram_token(
    app: AppHandle,
    token: String,
) -> Result<TelegramBotInfo, String> {
    let bot = get_bot_info(&app, &token).await?;
    Ok(TelegramBotInfo {
        username: bot.username.unwrap_or_else(|| bot.first_name.clone()),
        first_name: bot.first_name,
    })
}
