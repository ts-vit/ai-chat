// Telegram Bot API client — long polling, message handling, markdown conversion
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Emitter, Manager};
// tauri_plugin_store::StoreExt used via full path
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::AgentCancelTokens;
use crate::services::http_client::build_http_client;
use crate::services::mcp_manager::McpManager;
use crate::services::memory_vector_store::MemoryVectorStore;

type Pool = sqlx::SqlitePool;

const STORE_NAME: &str = "settings.json";
const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";
const MAX_MESSAGE_LENGTH: usize = 4096;
const TELEGRAM_SYSTEM_PROMPT: &str = r#"You are a personal AI assistant communicating via Telegram. Keep responses concise and well-structured. Use short paragraphs.

Key behaviors:
- Proactively save important information using memory_save (dates, facts, decisions, reminders, preferences)
- When searching memory, be thorough — the user relies on you to remember things
- Keep responses under 2000 characters when possible (Telegram messages should be digestible)
- Use Markdown formatting: **bold**, *italic*, `code`, ```code blocks```

When you see [CONTEXT_LIMIT_WARNING]:
- Immediately inform the user that the conversation context is almost full
- Save any unsaved important information from recent messages to memory
- Suggest starting a new session with /new command
- Explain that memory persists across sessions — nothing important will be lost"#;

// ─── Telegram API types ───────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TelegramUser {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramBotUser {
    pub username: Option<String>,
    pub first_name: String,
}

// ─── Event payloads ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramAuthRequestPayload {
    pub user_id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramStatusPayload {
    pub running: bool,
    pub bot_username: Option<String>,
}

// ─── TelegramBotManager ──────────────────────────────────────

struct TelegramBotRunState {
    polling_handle: JoinHandle<()>,
    cancel_token: CancellationToken,
    bot_username: String,
}

pub struct TelegramBotManager {
    state: Mutex<Option<TelegramBotRunState>>,
}

impl TelegramBotManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    pub async fn start(
        self: &Arc<Self>,
        app: AppHandle,
        pool: Pool,
        token: String,
    ) -> Result<String, String> {
        // Stop existing bot if running
        self.stop(&app).await;

        // Verify token first
        let bot_info = get_bot_info(&app, &token).await?;
        let bot_username = bot_info.username.unwrap_or_else(|| bot_info.first_name.clone());

        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        let app_clone = app.clone();
        let token_clone = token.clone();
        let username_clone = bot_username.clone();

        let handle = tokio::spawn(async move {
            polling_loop(app_clone, pool, token_clone, cancel_clone).await;
        });

        *self.state.lock().await = Some(TelegramBotRunState {
            polling_handle: handle,
            cancel_token,
            bot_username: username_clone.clone(),
        });

        let _ = app.emit("telegram-status-changed", TelegramStatusPayload {
            running: true,
            bot_username: Some(username_clone.clone()),
        });

        log::info!("[telegram] bot started: @{}", username_clone);
        Ok(username_clone)
    }

    pub async fn stop(&self, app: &AppHandle) {
        let mut state = self.state.lock().await;
        if let Some(run_state) = state.take() {
            run_state.cancel_token.cancel();
            run_state.polling_handle.abort();
            let _ = app.emit("telegram-status-changed", TelegramStatusPayload {
                running: false,
                bot_username: None,
            });
            log::info!("[telegram] bot stopped");
        }
    }

    pub async fn is_running(&self) -> bool {
        self.state.lock().await.is_some()
    }

    pub async fn bot_username(&self) -> Option<String> {
        self.state.lock().await.as_ref().map(|s| s.bot_username.clone())
    }
}

// ─── Bot API helpers ─────────────────────────────────────────

pub async fn get_bot_info(app: &AppHandle, token: &str) -> Result<TelegramBotUser, String> {
    let client = build_http_client(app, Some(Duration::from_secs(10))).await?;
    let url = format!("{}{}/getMe", TELEGRAM_API_BASE, token);
    let resp: TelegramResponse<TelegramBotUser> = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Telegram API error: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse getMe response: {}", e))?;

    if !resp.ok {
        return Err(format!("Telegram API error: {}", resp.description.unwrap_or_default()));
    }
    resp.result.ok_or_else(|| "No bot info in response".to_string())
}

pub async fn send_message(app: &AppHandle, token: &str, chat_id: i64, text: &str, parse_mode: Option<&str>) -> Result<(), String> {
    let client = build_http_client(app, Some(Duration::from_secs(30))).await?;
    let chunks = split_message(text);

    for chunk in &chunks {
        let url = format!("{}{}/sendMessage", TELEGRAM_API_BASE, token);
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": chunk,
        });
        if let Some(pm) = parse_mode {
            body["parse_mode"] = serde_json::Value::String(pm.to_string());
        }

        let resp = client.post(&url).json(&body).send().await;
        match resp {
            Ok(r) => {
                let status = r.status();
                if !status.is_success() {
                    let err_body = r.text().await.unwrap_or_default();
                    // If HTML parse mode failed, retry without parse mode
                    if parse_mode.is_some() && err_body.contains("can't parse entities") {
                        log::warn!("[telegram] HTML parse failed, retrying as plain text");
                        let plain_body = serde_json::json!({
                            "chat_id": chat_id,
                            "text": strip_html(chunk),
                        });
                        let _ = client.post(&format!("{}{}/sendMessage", TELEGRAM_API_BASE, token))
                            .json(&plain_body)
                            .send()
                            .await;
                    } else {
                        log::error!("[telegram] sendMessage error {}: {}", status, err_body);
                    }
                }
            }
            Err(e) => {
                log::error!("[telegram] sendMessage network error: {}", e);
            }
        }

        // Small delay between chunks to avoid rate limiting
        if chunks.len() > 1 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
    Ok(())
}

async fn send_typing(app: &AppHandle, token: &str, chat_id: i64) {
    let client = match build_http_client(app, Some(Duration::from_secs(5))).await {
        Ok(c) => c,
        Err(_) => return,
    };
    let url = format!("{}{}/sendChatAction", TELEGRAM_API_BASE, token);
    let _ = client.post(&url).json(&serde_json::json!({
        "chat_id": chat_id,
        "action": "typing",
    })).send().await;
}

// ─── Long polling loop ───────────────────────────────────────

async fn polling_loop(app: AppHandle, pool: Pool, token: String, cancel_token: CancellationToken) {
    let mut offset: i64 = 0;
    let mut first_batch = true;

    loop {
        if cancel_token.is_cancelled() {
            break;
        }

        let client = match build_http_client(&app, Some(Duration::from_secs(35))).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("[telegram] failed to build HTTP client: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let url = format!("{}{}/getUpdates", TELEGRAM_API_BASE, token);
        let body = serde_json::json!({
            "offset": offset,
            "timeout": 30,
            "allowed_updates": ["message"],
        });

        let resp = tokio::select! {
            r = client.post(&url).json(&body).send() => r,
            _ = cancel_token.cancelled() => break,
        };

        match resp {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let err = response.text().await.unwrap_or_default();
                    log::error!("[telegram] getUpdates error {}: {}", status, err);
                    let _ = app.emit("telegram-error", serde_json::json!({ "error": format!("{}: {}", status, err) }));
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let updates: TelegramResponse<Vec<TelegramUpdate>> = match response.json().await {
                    Ok(u) => u,
                    Err(e) => {
                        log::error!("[telegram] failed to parse updates: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                };

                if !updates.ok {
                    log::error!("[telegram] getUpdates not ok: {:?}", updates.description);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }

                let update_list = updates.result.unwrap_or_default();

                // On first batch (startup), handle queued messages
                if first_batch && !update_list.is_empty() {
                    first_batch = false;
                    let text_messages: Vec<&TelegramUpdate> = update_list.iter()
                        .filter(|u| u.message.as_ref().and_then(|m| m.text.as_ref()).is_some())
                        .collect();

                    if text_messages.len() > 5 {
                        // Skip older messages, notify user
                        let skipped = text_messages.len() - 5;
                        if let Some(last_msg) = text_messages.last() {
                            if let Some(ref msg) = last_msg.message {
                                let _ = send_message(&app, &token, msg.chat.id,
                                    &format!("⏩ Пропущено {} сообщений (бот был оффлайн). Обработаны последние сообщения.", skipped),
                                    None
                                ).await;
                            }
                        }

                        // Process only last 5 text messages
                        for update in text_messages.iter().skip(skipped) {
                            if cancel_token.is_cancelled() { break; }
                            if let Some(ref msg) = update.message {
                                process_message(&app, &pool, &token, msg, &cancel_token).await;
                            }
                        }

                        // Update offset to skip all
                        if let Some(last) = update_list.last() {
                            offset = last.update_id + 1;
                        }
                        continue;
                    }
                }

                for update in &update_list {
                    offset = update.update_id + 1;

                    if cancel_token.is_cancelled() { break; }

                    if let Some(ref msg) = update.message {
                        process_message(&app, &pool, &token, msg, &cancel_token).await;
                    }
                }
            }
            Err(e) => {
                log::error!("[telegram] getUpdates network error: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
    log::info!("[telegram] polling loop exited");
}

// ─── Message processing ──────────────────────────────────────

async fn process_message(app: &AppHandle, pool: &Pool, token: &str, msg: &TelegramMessage, cancel_token: &CancellationToken) {
    let text = match &msg.text {
        Some(t) => t.clone(),
        None => return, // ignore non-text messages
    };

    let from = match &msg.from {
        Some(u) => u.clone(),
        None => return,
    };

    let telegram_chat_id = msg.chat.id;

    // Read auth settings
    let store = match tauri_plugin_store::StoreExt::store(app, STORE_NAME) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[telegram] failed to open store: {}", e);
            return;
        }
    };

    let authorized_user_id: Option<i64> = store.get("telegramUserId")
        .and_then(|v| v.as_i64());

    // Auth check
    match authorized_user_id {
        None => {
            // First contact — emit auth request
            let _ = app.emit("telegram-auth-request", TelegramAuthRequestPayload {
                user_id: from.id,
                first_name: from.first_name.clone(),
                last_name: from.last_name.clone(),
                username: from.username.clone(),
            });
            let _ = send_message(app, token, telegram_chat_id,
                "⏳ Ожидание авторизации. Подтвердите доступ в приложении UNI AI.", None).await;

            let _ = app.emit("telegram-message-received", serde_json::json!({
                "text": format!("Auth request from {} ({})", from.first_name, from.id),
            }));
            return;
        }
        Some(uid) if uid != from.id => {
            let _ = send_message(app, token, telegram_chat_id, "⛔ Бот недоступен.", None).await;
            return;
        }
        _ => {} // authorized
    }

    let _ = app.emit("telegram-message-received", serde_json::json!({ "text": text }));

    // Handle commands
    if text.starts_with('/') {
        match text.trim() {
            "/new" => {
                handle_new_command(app, pool, token, telegram_chat_id).await;
                return;
            }
            "/status" => {
                handle_status_command(app, pool, token, telegram_chat_id).await;
                return;
            }
            "/start" => {
                let _ = send_message(app, token, telegram_chat_id,
                    "👋 Привет! Я UNI AI Assistant. Напишите мне что угодно, и я помогу.\n\nКоманды:\n/new — новая сессия\n/status — статус",
                    None
                ).await;
                return;
            }
            _ => {} // process as regular message
        }
    }

    // Send typing indicator
    send_typing(app, token, telegram_chat_id).await;

    // Handle regular message — run through agent
    handle_agent_message(app, pool, token, telegram_chat_id, &text, cancel_token).await;
}

async fn handle_new_command(app: &AppHandle, pool: &Pool, token: &str, telegram_chat_id: i64) {
    let now = uni_common::now_unix_secs();

    // Archive current Telegram chat
    let _ = sqlx::query(
        "UPDATE chats SET is_telegram_chat = 0, title = title || ' — ' || ? WHERE is_telegram_chat = 1 AND mode = 'assistant'"
    )
    .bind(chrono::Utc::now().format("%Y-%m-%d").to_string())
    .execute(pool)
    .await;

    // Create new chat
    let chat_id = uni_common::generate_id();
    let _ = sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, mode, is_telegram_chat, provider_id) VALUES (?, 'Telegram', ?, ?, 'assistant', 1, 'openrouter')"
    )
    .bind(&chat_id).bind(now).bind(now)
    .execute(pool)
    .await;

    let _ = send_message(app, token, telegram_chat_id,
        "✅ Новая сессия начата. Память сохранена.", None).await;
}

async fn handle_status_command(app: &AppHandle, pool: &Pool, token: &str, telegram_chat_id: i64) {
    let store = match tauri_plugin_store::StoreExt::store(app, STORE_NAME) {
        Ok(s) => s,
        Err(_) => return,
    };

    let telegram_model = store.get("telegramModel")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty());
    let model = match &telegram_model {
        Some(m) => m.clone(),
        None => store.get("model")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let model_suffix = if telegram_model.is_none() { " (авто)" } else { "" };

    let memory_count: i64 = sqlx::query("SELECT COUNT(*) as cnt FROM agent_memory")
        .fetch_one(pool)
        .await
        .map(|row| row.try_get("cnt").unwrap_or(0))
        .unwrap_or(0);

    // Count messages in active Telegram chat
    let msg_count: i64 = sqlx::query(
        "SELECT COUNT(*) as cnt FROM messages WHERE chat_id IN (SELECT id FROM chats WHERE is_telegram_chat = 1 AND mode = 'assistant')"
    )
    .fetch_one(pool)
    .await
    .map(|row| row.try_get("cnt").unwrap_or(0))
    .unwrap_or(0);

    let status_text = format!(
        "📊 Статус UNI AI\n\nМодель: {}{}\nСообщений в сессии: {}\nЗаписей в памяти: {}",
        model, model_suffix, msg_count, memory_count
    );

    let _ = send_message(app, token, telegram_chat_id, &status_text, None).await;
}

async fn handle_agent_message(
    app: &AppHandle,
    pool: &Pool,
    token: &str,
    telegram_chat_id: i64,
    text: &str,
    _cancel_token: &CancellationToken,
) {
    // Find or create active Telegram chat
    let chat_id = match find_or_create_telegram_chat(pool).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("[telegram] failed to find/create chat: {}", e);
            let _ = send_message(app, token, telegram_chat_id, "❌ Ошибка: не удалось создать чат.", None).await;
            return;
        }
    };

    // Read credentials from settings
    let store = match tauri_plugin_store::StoreExt::store(app, STORE_NAME) {
        Ok(s) => s,
        Err(e) => {
            log::error!("[telegram] store error: {}", e);
            let _ = send_message(app, token, telegram_chat_id, "❌ Ошибка настроек.", None).await;
            return;
        }
    };

    let api_key = store.get("api_key")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let telegram_model = store.get("telegramModel")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty());
    let model = telegram_model.unwrap_or_else(|| {
        store.get("model")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string())
    });

    if api_key.is_empty() {
        let _ = send_message(app, token, telegram_chat_id,
            "❌ API ключ не настроен. Откройте UNI AI и настройте OpenRouter API ключ.", None).await;
        return;
    }

    // Prepare agent run
    let run_id = uni_common::generate_id();
    let user_msg_id = uni_common::generate_id();
    let assistant_msg_id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();

    // Find last message in chat for parent_id
    let parent_id: Option<String> = sqlx::query(
        "SELECT id FROM messages WHERE chat_id = ? ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(&chat_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| row.get("id"));

    // Create agent_run record
    if let Err(e) = sqlx::query(
        "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at) VALUES (?, ?, 'running', 0, 25, ?)"
    )
    .bind(&run_id).bind(&chat_id).bind(now)
    .execute(pool).await {
        log::error!("[telegram] failed to create agent_run: {}", e);
        let _ = send_message(app, token, telegram_chat_id, "❌ Ошибка запуска агента.", None).await;
        return;
    }

    // Save user message to DB
    if let Err(e) = sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'user', ?, ?, ?, '', 0, 0, 0.0, 0, 0, ?)"
    )
    .bind(&user_msg_id).bind(&chat_id).bind(text).bind(&parent_id).bind(now).bind(&run_id)
    .execute(pool).await {
        log::error!("[telegram] failed to save user message: {}", e);
        let _ = send_message(app, token, telegram_chat_id, "❌ Ошибка сохранения сообщения.", None).await;
        return;
    }

    // Update active_child_map for branching
    if let Some(ref pid) = parent_id {
        let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_one(pool)
            .await
            .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|_| "{}".to_string());
        let mut map: std::collections::HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
        map.insert(pid.clone(), user_msg_id.clone());
        let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
        let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
            .bind(&new_map).bind(&chat_id)
            .execute(pool).await;
    }

    // Set up oneshot channel for completion notification
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    {
        if let Some(notifier) = app.try_state::<crate::TelegramRunNotifier>() {
            let mut map = notifier.write().await;
            map.insert(run_id.clone(), tx);
        }
    }

    // Get managed state for agent_loop
    let mcp_mgr: Arc<McpManager> = app.state::<Arc<McpManager>>().inner().clone();
    let mem_store: Arc<Option<MemoryVectorStore>> = app.state::<Arc<Option<MemoryVectorStore>>>().inner().clone();
    let kb_store: Arc<Option<crate::services::kb_vector_store::KbVectorStore>> = app.state::<Arc<Option<crate::services::kb_vector_store::KbVectorStore>>>().inner().clone();
    let cancel_tokens: AgentCancelTokens = app.state::<AgentCancelTokens>().inner().clone();

    // Create cancel token for this run
    let run_cancel_token = CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(run_id.clone(), run_cancel_token.clone());
    }

    // Build system prompt with Telegram context
    let system_prompt = Some(TELEGRAM_SYSTEM_PROMPT.to_string());

    // Spawn agent_loop
    let app_clone = app.clone();
    let pool_clone = pool.clone();
    let cancel_tokens_clone = cancel_tokens.clone();
    let run_id_for_query = run_id.clone();
    let chat_id_for_query = chat_id.clone();

    tokio::spawn(async move {
        crate::commands::agent::agent_loop(
            app_clone,
            pool_clone,
            mcp_mgr,
            mem_store,
            kb_store,
            chat_id,
            run_id,
            model,
            None, // base_url — default OpenRouter
            api_key,
            system_prompt,
            user_msg_id,
            assistant_msg_id,
            25, // max_iterations
            true, // auto_mode
            run_cancel_token,
            cancel_tokens_clone,
            None, // temperature
            None, // max_tokens
            None, // top_p
            None, // top_k
            None, // frequency_penalty
            None, // presence_penalty
            None, // supports_tool_use
            None, // scope_agent_run_id
            None, // plan_id
            0,    // depth
        ).await;
    });

    // Wait for agent completion with timeout
    let status = match tokio::time::timeout(Duration::from_secs(300), rx).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => {
            log::error!("[telegram] oneshot channel dropped");
            "failed".to_string()
        }
        Err(_) => {
            log::error!("[telegram] agent timed out after 5 minutes");
            "timeout".to_string()
        }
    };

    if status == "failed" || status == "timeout" {
        let _ = send_message(app, token, telegram_chat_id,
            "❌ Агент не смог завершить обработку. Попробуйте ещё раз.", None).await;
        return;
    }

    // Fetch the last assistant message
    let assistant_content = sqlx::query(
        "SELECT content FROM messages WHERE chat_id = ? AND role = 'assistant' AND agent_run_id = ? ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(&chat_id_for_query)
    .bind(&run_id_for_query)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| row.get::<String, _>("content"))
    .unwrap_or_else(|| "Агент завершил работу без ответа.".to_string());

    // Convert to Telegram HTML and send
    let html_content = convert_to_telegram_html(&assistant_content);
    let _ = send_message(app, token, telegram_chat_id, &html_content, Some("HTML")).await;
}

// ─── Chat management ─────────────────────────────────────────

async fn find_or_create_telegram_chat(pool: &Pool) -> Result<String, String> {
    // Find active Telegram chat
    let existing: Option<String> = sqlx::query(
        "SELECT id FROM chats WHERE is_telegram_chat = 1 AND mode = 'assistant' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .map(|row| row.get("id"));

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create new Telegram chat
    let chat_id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, mode, is_telegram_chat, provider_id) VALUES (?, 'Telegram', ?, ?, 'assistant', 1, 'openrouter')"
    )
    .bind(&chat_id).bind(now).bind(now)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create telegram chat: {}", e))?;

    Ok(chat_id)
}

// ─── Markdown → Telegram HTML conversion ─────────────────────

pub fn convert_to_telegram_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_code_block = false;
    let mut in_inline_code = false;

    while i < len {
        // Code blocks: ```...```
        if !in_inline_code && i + 2 < len && chars[i] == '`' && chars[i + 1] == '`' && chars[i + 2] == '`' {
            if in_code_block {
                result.push_str("</pre>");
                in_code_block = false;
                i += 3;
                // Skip trailing newline
                if i < len && chars[i] == '\n' { i += 1; }
            } else {
                in_code_block = true;
                i += 3;
                // Skip language identifier and newline
                while i < len && chars[i] != '\n' { i += 1; }
                if i < len { i += 1; } // skip \n
                result.push_str("<pre>");
            }
            continue;
        }

        if in_code_block {
            match chars[i] {
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '&' => result.push_str("&amp;"),
                c => result.push(c),
            }
            i += 1;
            continue;
        }

        // Inline code: `...`
        if chars[i] == '`' {
            if in_inline_code {
                result.push_str("</code>");
                in_inline_code = false;
            } else {
                in_inline_code = true;
                result.push_str("<code>");
            }
            i += 1;
            continue;
        }

        if in_inline_code {
            match chars[i] {
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '&' => result.push_str("&amp;"),
                c => result.push(c),
            }
            i += 1;
            continue;
        }

        // Bold: **...**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            // Find closing **
            if let Some(end) = find_closing_marker(&chars, i + 2, &['*', '*']) {
                result.push_str("<b>");
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&escape_html(&inner));
                result.push_str("</b>");
                i = end + 2;
                continue;
            }
        }

        // Italic: *...*  (single asterisk, not followed by another)
        if chars[i] == '*' && (i + 1 >= len || chars[i + 1] != '*') {
            if let Some(end) = find_closing_single(&chars, i + 1, '*') {
                result.push_str("<i>");
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&escape_html(&inner));
                result.push_str("</i>");
                i = end + 1;
                continue;
            }
        }

        // HTML escape for regular text
        match chars[i] {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            c => result.push(c),
        }
        i += 1;
    }

    // Close unclosed tags
    if in_inline_code { result.push_str("</code>"); }
    if in_code_block { result.push_str("</pre>"); }

    result
}

fn find_closing_marker(chars: &[char], start: usize, marker: &[char]) -> Option<usize> {
    let mlen = marker.len();
    let limit = chars.len().saturating_sub(mlen - 1);
    for i in start..limit {
        if chars[i] == '\n' && (i + 1 < chars.len() && chars[i + 1] == '\n') {
            return None; // don't span across paragraphs
        }
        let mut matched = true;
        for j in 0..mlen {
            if chars[i + j] != marker[j] { matched = false; break; }
        }
        if matched { return Some(i); }
    }
    None
}

fn find_closing_single(chars: &[char], start: usize, marker: char) -> Option<usize> {
    for i in start..chars.len() {
        if chars[i] == '\n' && (i + 1 < chars.len() && chars[i + 1] == '\n') {
            return None;
        }
        if chars[i] == marker {
            return Some(i);
        }
    }
    None
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn strip_html(text: &str) -> String {
    // Simple HTML tag stripper for fallback
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        if ch == '<' { in_tag = true; continue; }
        if ch == '>' { in_tag = false; continue; }
        if !in_tag { result.push(ch); }
    }
    result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

// ─── Message splitting ───────────────────────────────────────

fn split_message(text: &str) -> Vec<String> {
    if text.len() <= MAX_MESSAGE_LENGTH {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= MAX_MESSAGE_LENGTH {
            chunks.push(remaining.to_string());
            break;
        }

        // Try to split at a newline boundary
        let search_end = MAX_MESSAGE_LENGTH.min(remaining.len());
        // Find safe char boundary
        let mut safe_end = search_end;
        while safe_end > 0 && !remaining.is_char_boundary(safe_end) {
            safe_end -= 1;
        }

        let split_at = remaining[..safe_end]
            .rfind('\n')
            .map(|pos| pos + 1) // include the newline
            .unwrap_or_else(|| {
                // No newline found, split at safe char boundary
                safe_end
            });

        chunks.push(remaining[..split_at].to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_short_message_single() {
        let text = "Hello world";
        let chunks = split_message(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world");
    }

    #[test]
    fn test_split_exact_limit() {
        let text = "a".repeat(MAX_MESSAGE_LENGTH);
        let chunks = split_message(&text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn test_split_long_message() {
        let text = "a".repeat(MAX_MESSAGE_LENGTH * 3);
        let chunks = split_message(&text);
        assert!(chunks.len() >= 3);
        for c in &chunks {
            assert!(c.len() <= MAX_MESSAGE_LENGTH);
        }
    }

    #[test]
    fn test_split_preserves_content() {
        let text = "Hello world. ".repeat(500); // ~6500 chars, will need splitting
        let chunks = split_message(&text);
        let joined: String = chunks.concat();
        assert_eq!(joined, text);
    }

    #[test]
    fn test_split_russian_utf8_safe() {
        // "Привет " is 13 bytes (6 Cyrillic chars × 2 bytes + 1 space)
        // Fill enough to exceed 4096 bytes
        let text = "Привет ".repeat(500); // ~6500 bytes
        let chunks = split_message(&text);
        assert!(chunks.len() >= 2);
        for c in &chunks {
            assert!(c.len() <= MAX_MESSAGE_LENGTH);
            // Verify valid UTF-8 (String guarantees this, but check char boundary)
            assert!(c.is_char_boundary(c.len()));
        }
    }

    #[test]
    fn test_split_prefers_newline_boundary() {
        // Create text with newlines before the 4096 limit
        let mut text = String::new();
        for i in 0..200 {
            text.push_str(&format!("Line {}\n", i));
        }
        let chunks = split_message(&text);
        // First chunk should end at a newline
        if chunks.len() > 1 {
            assert!(chunks[0].ends_with('\n'));
        }
    }

    #[test]
    fn test_split_no_newlines() {
        let text = "a".repeat(MAX_MESSAGE_LENGTH + 100);
        let chunks = split_message(&text);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn test_split_empty_string() {
        let chunks = split_message("");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "");
    }
}
