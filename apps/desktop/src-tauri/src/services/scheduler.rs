use std::sync::Arc;
use std::time::Duration;
use chrono::{Local, TimeZone};
use cron::Schedule;
use sqlx::Row;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uni_settings::{JsonSettingsStore, SettingsStore};

use crate::models::scheduler::ScheduledTask;
use crate::services::mcp_manager::McpManager;
use crate::services::memory_vector_store::MemoryVectorStore;
use crate::{AgentCancelTokens, TelegramRunNotifier};

type Pool = sqlx::SqlitePool;
type SettingsState = Arc<JsonSettingsStore>;

struct SchedulerRunState {
    handle: JoinHandle<()>,
    cancel_token: CancellationToken,
}

pub struct SchedulerManager {
    state: Mutex<Option<SchedulerRunState>>,
}

impl SchedulerManager {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
        }
    }

    pub async fn start(&self, app: AppHandle, pool: Pool) {
        let mut state = self.state.lock().await;
        if state.is_some() {
            return; // already running
        }
        let cancel_token = CancellationToken::new();
        let cancel_clone = cancel_token.clone();
        let handle = tokio::spawn(async move {
            scheduler_loop(app, pool, cancel_clone).await;
        });
        *state = Some(SchedulerRunState {
            handle,
            cancel_token,
        });
        log::info!("[scheduler] started");
    }

    pub async fn stop(&self) {
        let mut state = self.state.lock().await;
        if let Some(run_state) = state.take() {
            run_state.cancel_token.cancel();
            run_state.handle.abort();
            log::info!("[scheduler] stopped");
        }
    }

    pub async fn is_running(&self) -> bool {
        self.state.lock().await.is_some()
    }
}

async fn scheduler_loop(app: AppHandle, pool: Pool, cancel_token: CancellationToken) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                log::info!("[scheduler] cancelled");
                break;
            }
            _ = interval.tick() => {
                if let Err(e) = check_and_execute_tasks(&app, &pool).await {
                    log::error!("[scheduler] tick error: {}", e);
                }
            }
        }
    }
}

async fn check_and_execute_tasks(app: &AppHandle, pool: &Pool) -> Result<(), String> {
    let now = uni_common::now_unix_secs();

    let rows = sqlx::query(
        "SELECT * FROM scheduled_tasks WHERE enabled = 1 AND next_run_at IS NOT NULL AND next_run_at <= ?"
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query error: {}", e))?;

    for row in rows {
        let task = row_to_task(&row);
        if let Err(e) = execute_task(app, pool, &task).await {
            log::error!("[scheduler] task '{}' failed: {}", task.name, e);
            // Update task status to failed
            let _ = sqlx::query(
                "UPDATE scheduled_tasks SET last_run_status = 'failed', last_run_error = ?, updated_at = ? WHERE id = ?"
            )
            .bind(&e)
            .bind(now)
            .bind(&task.id)
            .execute(pool)
            .await;

            // Emit failure event
            let _ = app.emit("scheduler-task-failed", serde_json::json!({
                "taskId": task.id,
                "taskName": task.name,
                "error": e,
            }));
        }

        // Compute and set next_run_at
        if let Some(next) = compute_next_run(&task.cron_expression) {
            let _ = sqlx::query(
                "UPDATE scheduled_tasks SET next_run_at = ?, updated_at = ? WHERE id = ?"
            )
            .bind(next)
            .bind(now)
            .bind(&task.id)
            .execute(pool)
            .await;
        }
    }

    Ok(())
}

async fn execute_task(app: &AppHandle, pool: &Pool, task: &ScheduledTask) -> Result<(), String> {
    let now = uni_common::now_unix_secs();

    // Mark as running
    sqlx::query(
        "UPDATE scheduled_tasks SET last_run_status = 'running', last_run_at = ?, last_run_error = NULL, updated_at = ? WHERE id = ?"
    )
    .bind(now).bind(now).bind(&task.id)
    .execute(pool)
    .await
    .map_err(|e| format!("update running status: {}", e))?;

    // Emit started event
    let _ = app.emit("scheduler-task-started", serde_json::json!({
        "taskId": task.id,
        "taskName": task.name,
    }));

    // Read credentials from store
    let settings = app.state::<SettingsState>();

    let api_key = settings.get("llm.openrouter.api_key")
        .await
        .unwrap_or_default()
        .unwrap_or_default();

    if api_key.is_empty() {
        return Err("API key not configured".to_string());
    }

    let model = if let Some(ref m) = task.model {
        if !m.is_empty() {
            m.clone()
        } else {
            settings.get("llm.openrouter.model")
                .await
                .unwrap_or_default()
                .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string())
        }
    } else {
        settings.get("llm.openrouter.model")
            .await
            .unwrap_or_default()
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".to_string())
    };

    // Find or create chat for this task
    let chat_id = find_or_create_task_chat(pool, task, now).await?;

    // Context rotation: check message count
    let msg_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE chat_id = ?"
    )
    .bind(&chat_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let chat_id = if msg_count > 100 {
        // Archive old chat
        let archive_title = format!("{} — {}", task.name, Local::now().format("%Y-%m-%d"));
        let _ = sqlx::query("UPDATE chats SET title = ?, scheduled_task_id = NULL WHERE id = ?")
            .bind(&archive_title).bind(&chat_id)
            .execute(pool).await;
        // Create new chat
        create_task_chat(pool, task, now).await?
    } else {
        chat_id
    };

    // Attach skill if specified
    if let Some(ref skill_id) = task.skill_id {
        let already: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM chat_skills WHERE chat_id = ? AND skill_id = ?"
        )
        .bind(&chat_id).bind(skill_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !already {
            let _ = sqlx::query(
                "INSERT INTO chat_skills (chat_id, skill_id, attached_by) VALUES (?, ?, 'scheduler')"
            )
            .bind(&chat_id).bind(skill_id)
            .execute(pool).await;
        }
    }

    // Prepare agent run
    let run_id = uni_common::generate_id();
    let user_msg_id = uni_common::generate_id();
    let assistant_msg_id = uni_common::generate_id();

    // Find parent message
    let parent_id: Option<String> = sqlx::query("SELECT id FROM messages WHERE chat_id = ? ORDER BY timestamp DESC LIMIT 1")
        .bind(&chat_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|row| row.get("id"));

    // Create agent_run
    sqlx::query(
        "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at, assigned_model) VALUES (?, ?, 'running', 0, 25, ?, ?)"
    )
    .bind(&run_id).bind(&chat_id).bind(now).bind(&model)
    .execute(pool)
    .await
    .map_err(|e| format!("create agent_run: {}", e))?;

    // Save user message
    sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'user', ?, ?, ?, '', 0, 0, 0.0, 0, 0, ?)"
    )
    .bind(&user_msg_id).bind(&chat_id).bind(&task.prompt).bind(&parent_id).bind(now).bind(&run_id)
    .execute(pool)
    .await
    .map_err(|e| format!("save user message: {}", e))?;

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

    // Set up oneshot channel for completion
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    {
        if let Some(notifier) = app.try_state::<TelegramRunNotifier>() {
            let mut map = notifier.write().await;
            map.insert(run_id.clone(), tx);
        }
    }

    // Get managed state
    let mcp_mgr: Arc<McpManager> = app.state::<Arc<McpManager>>().inner().clone();
    let mem_store: Arc<Option<MemoryVectorStore>> = app.state::<Arc<Option<MemoryVectorStore>>>().inner().clone();
    let kb_store: Arc<Option<crate::services::kb_vector_store::KbVectorStore>> = app.state::<Arc<Option<crate::services::kb_vector_store::KbVectorStore>>>().inner().clone();
    let cancel_tokens: AgentCancelTokens = app.state::<AgentCancelTokens>().inner().clone();

    let run_cancel_token = CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(run_id.clone(), run_cancel_token.clone());
    }

    // Spawn agent_loop
    let app_clone = app.clone();
    let pool_clone = pool.clone();
    let cancel_tokens_clone = cancel_tokens.clone();
    let run_id_for_spawn = run_id.clone();
    let chat_id_for_spawn = chat_id.clone();
    let run_id_for_query = run_id.clone();
    let chat_id_for_query = chat_id.clone();
    let task_name = task.name.clone();
    let task_id = task.id.clone();
    let deliver_telegram = task.deliver_telegram;
    let deliver_desktop = task.deliver_desktop_notification;
    let cron_expr = task.cron_expression.clone();

    tokio::spawn(async move {
        crate::commands::agent::agent_loop(
            app_clone.clone(),
            pool_clone.clone(),
            mcp_mgr,
            mem_store,
            kb_store,
            chat_id_for_spawn,
            run_id_for_spawn,
            model,
            None,  // base_url
            api_key,
            None,  // system_prompt — agent builds its own
            user_msg_id,
            assistant_msg_id,
            25,    // max_iterations
            true,  // auto_mode
            run_cancel_token,
            cancel_tokens_clone,
            None,  // temperature
            None,  // max_tokens
            None,  // top_p
            None,  // top_k
            None,  // frequency_penalty
            None,  // presence_penalty
            None,  // supports_tool_use
            None,  // scope_agent_run_id
            None,  // plan_id
            0,     // depth
        ).await;
    });

    // Wait for completion with 10-minute timeout
    let status = match tokio::time::timeout(Duration::from_secs(600), rx).await {
        Ok(Ok(s)) => s,
        Ok(Err(_)) => {
            log::error!("[scheduler] oneshot channel dropped for task '{}'", task_name);
            "failed".to_string()
        }
        Err(_) => {
            log::error!("[scheduler] task '{}' timed out after 10 minutes", task_name);
            "timeout".to_string()
        }
    };

    if status == "failed" || status == "timeout" {
        // Update status
        let _ = sqlx::query(
            "UPDATE scheduled_tasks SET last_run_status = 'failed', last_run_error = ?, run_count = run_count + 1, updated_at = ? WHERE id = ?"
        )
        .bind(format!("Agent {}", status))
        .bind(now)
        .bind(&task_id)
        .execute(pool)
        .await;

        let _ = app.emit("scheduler-task-failed", serde_json::json!({
            "taskId": task_id,
            "taskName": task_name,
            "error": format!("Agent {}", status),
        }));

        return Err(format!("Agent {}", status));
    }

    // Fetch last assistant message
    let assistant_content: String = sqlx::query_scalar(
        "SELECT content FROM messages WHERE chat_id = ? AND role = 'assistant' AND agent_run_id = ? ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(&chat_id_for_query)
    .bind(&run_id_for_query)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "Агент завершил работу без ответа.".to_string());

    // Update task status
    let _ = sqlx::query(
        "UPDATE scheduled_tasks SET last_run_status = 'completed', last_run_error = NULL, run_count = run_count + 1, updated_at = ? WHERE id = ?"
    )
    .bind(now)
    .bind(&task_id)
    .execute(pool)
    .await;

    // Deliver results
    let result_preview = if assistant_content.len() > 200 {
        let boundary = assistant_content.char_indices()
            .take_while(|(i, _)| *i < 200)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(200);
        format!("{}...", &assistant_content[..boundary])
    } else {
        assistant_content.clone()
    };

    // Desktop notification
    if deliver_desktop {
        let _ = app.emit("scheduler-task-completed", serde_json::json!({
            "taskId": task_id,
            "taskName": task_name,
            "resultPreview": result_preview,
        }));
    }

    // Telegram delivery
    if deliver_telegram {
        deliver_to_telegram(app, &task_name, &assistant_content, &cron_expr).await;
    }

    Ok(())
}

async fn deliver_to_telegram(app: &AppHandle, task_name: &str, content: &str, cron_expression: &str) {
    // Check if Telegram bot is running
    let tg_mgr = match app.try_state::<Arc<crate::services::telegram_bot::TelegramBotManager>>() {
        Some(mgr) => mgr.inner().clone(),
        None => return,
    };

    if !tg_mgr.is_running().await {
        return;
    }

    // Get Telegram token and chat_id from store
    let settings = app.state::<SettingsState>();

    let token = settings.get("telegram.bot_token")
        .await
        .unwrap_or_default()
        .unwrap_or_default();
    let chat_id: i64 = settings.get("telegram.user_id")
        .await
        .unwrap_or_default()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    if token.is_empty() || chat_id == 0 {
        return;
    }

    // Format next run time
    let next_run_str = compute_next_run(cron_expression)
        .map(|ts| {
            Local.timestamp_opt(ts, 0)
                .single()
                .map(|dt| dt.format("%d.%m %H:%M").to_string())
                .unwrap_or_else(|| "—".to_string())
        })
        .unwrap_or_else(|| "—".to_string());

    let message = format!(
        "📋 <b>{}</b>\n\n{}\n\n<i>⏰ Следующий запуск: {}</i>",
        task_name, content, next_run_str
    );

    let _ = crate::services::telegram_bot::send_message(app, &token, chat_id, &message, Some("HTML")).await;
}

async fn find_or_create_task_chat(pool: &Pool, task: &ScheduledTask, now: i64) -> Result<String, String> {
    // Find existing chat for this task
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM chats WHERE scheduled_task_id = ? ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&task.id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("query chat: {}", e))?;

    if let Some(id) = existing {
        return Ok(id);
    }

    create_task_chat(pool, task, now).await
}

async fn create_task_chat(pool: &Pool, task: &ScheduledTask, now: i64) -> Result<String, String> {
    let chat_id = uni_common::generate_id();
    let project_id = task.project_id.as_deref();

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, mode, scheduled_task_id, project_id, web_search_enabled) VALUES (?, ?, ?, ?, 'assistant', ?, ?, 1)"
    )
    .bind(&chat_id)
    .bind(&task.name)
    .bind(now)
    .bind(now)
    .bind(&task.id)
    .bind(project_id)
    .execute(pool)
    .await
    .map_err(|e| format!("create chat: {}", e))?;

    Ok(chat_id)
}

pub fn compute_next_run(cron_expression: &str) -> Option<i64> {
    // The cron crate expects 7-field expressions (sec min hour dom mon dow year)
    // but we use standard 5-field (min hour dom mon dow).
    // Prepend "0 " for seconds and append " *" for year.
    let full_expr = format!("0 {} *", cron_expression);

    let schedule: Schedule = full_expr.parse().ok()?;
    let next = schedule.upcoming(Local).next()?;
    Some(next.timestamp())
}

fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> ScheduledTask {
    ScheduledTask {
        id: row.get("id"),
        name: row.get("name"),
        prompt: row.get("prompt"),
        cron_expression: row.get("cron_expression"),
        enabled: row.get::<i32, _>("enabled") != 0,
        mode: row.get("mode"),
        model: row.get("model"),
        skill_id: row.get("skill_id"),
        project_id: row.get("project_id"),
        deliver_telegram: row.get::<i32, _>("deliver_telegram") != 0,
        deliver_desktop_notification: row.get::<i32, _>("deliver_desktop_notification") != 0,
        last_run_at: row.get("last_run_at"),
        next_run_at: row.get("next_run_at"),
        last_run_status: row.get("last_run_status"),
        last_run_error: row.get("last_run_error"),
        run_count: row.get("run_count"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
