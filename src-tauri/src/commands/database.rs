// Команды для работы с SQLite (чаты и сообщения)
use std::sync::Arc;
use serde::Deserialize;
use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::commands::attachments::delete_attachments_for_message;
use crate::models::chat::{DbChat, DbMessage};
use crate::services::fts;
use crate::services::vector_store::VectorStore;

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn create_chat(
    pool: State<'_, Pool>,
    title: String,
    system_prompt: Option<String>,
    provider_id: Option<String>,
    model: Option<String>,
    is_image_model: Option<bool>,
) -> Result<DbChat, String> {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let system_prompt_str = system_prompt.unwrap_or_default();
    let provider = provider_id
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "openrouter".to_string());
    let model_str = model.unwrap_or_default();
    let is_image = is_image_model.unwrap_or(false) as i64;

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, system_prompt, provider_id, model, is_image_model) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(now)
    .bind(now)
    .bind(&system_prompt_str)
    .bind(&provider)
    .bind(&model_str)
    .bind(is_image)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[create_chat] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbChat {
        id: id.clone(),
        title,
        created_at: now,
        updated_at: now,
        system_prompt: Some(system_prompt_str.clone()).filter(|s| !s.is_empty()),
        provider_id: provider,
        model: model_str,
        folder_id: None,
        is_image_model: is_image_model.unwrap_or(false),
    })
}

#[tauri::command]
pub async fn delete_chat(app: AppHandle, pool: State<'_, Pool>, id: String) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, content FROM messages WHERE chat_id = ? AND has_attachments = 1")
        .bind(&id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_chat] SQL error (fetch): {}", e);
            e.to_string()
        })?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    sqlx::query("DELETE FROM chats WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_chat] SQL error (delete): {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_chats(pool: State<'_, Pool>) -> Result<Vec<DbChat>, String> {
    let rows = sqlx::query(
        "SELECT id, title, created_at, updated_at, system_prompt, provider_id, model, folder_id, is_image_model FROM chats ORDER BY updated_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[get_all_chats] SQL error: {}", e);
        e.to_string()
    })?;

    let chats = rows
        .into_iter()
        .map(|row| DbChat {
            id: row.get("id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            system_prompt: row.try_get::<String, _>("system_prompt").ok(),
            provider_id: row.try_get::<String, _>("provider_id").unwrap_or_else(|_| "openrouter".to_string()),
            model: row.try_get::<String, _>("model").unwrap_or_default(),
            folder_id: row.try_get::<String, _>("folder_id").ok(),
            is_image_model: row.try_get::<i64, _>("is_image_model").unwrap_or(0) != 0,
        })
        .collect();
    Ok(chats)
}

#[tauri::command]
pub async fn save_message(
    pool: State<'_, Pool>,
    chat_id: String,
    role: String,
    content: String,
    parent_id: Option<String>,
    timestamp: i64,
    model: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
) -> Result<DbMessage, String> {
    let id = Uuid::new_v4().to_string();
    let pt = prompt_tokens as i64;
    let ct = completion_tokens as i64;

    sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(&chat_id)
    .bind(&role)
    .bind(&content)
    .bind(&parent_id)
    .bind(timestamp)
    .bind(&model)
    .bind(pt)
    .bind(ct)
    .bind(cost)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[save_message] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbMessage {
        id: id.clone(),
        chat_id,
        role,
        content,
        parent_id,
        timestamp,
        model: if model.is_empty() { None } else { Some(model) },
        prompt_tokens: Some(pt),
        completion_tokens: Some(ct),
        cost: Some(cost),
        has_attachments: Some(0),
    })
}

#[tauri::command]
pub async fn update_message_usage(
    pool: State<'_, Pool>,
    id: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE messages SET prompt_tokens = ?, completion_tokens = ?, cost = ? WHERE id = ?",
    )
    .bind(prompt_tokens as i64)
    .bind(completion_tokens as i64)
    .bind(cost)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[update_message_usage] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

#[tauri::command]
pub async fn update_message_content(
    pool: State<'_, Pool>,
    id: String,
    content: String,
) -> Result<(), String> {
    let has_attachments = content
        .trim_start()
        .starts_with('[')
        .then(|| {
            serde_json::from_str::<Vec<serde_json::Value>>(&content)
                .ok()
                .map(|arr| arr.iter().any(|b| b.get("path").is_some()))
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if has_attachments {
        sqlx::query("UPDATE messages SET content = ?, has_attachments = 1 WHERE id = ?")
            .bind(&content)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                let msg = e.to_string();
                eprintln!("[update_message_content] SQL error (has_attachments): {}", msg);
                msg
            })?;
    } else {
        sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
            .bind(&content)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                let msg = e.to_string();
                eprintln!("[update_message_content] SQL error: {}", msg);
                msg
            })?;
    }
    Ok(())
}

/// Обновляет content сообщения и выставляет has_attachments = 1 (для сообщений с вложениями).
pub async fn update_message_content_with_attachments(
    pool: &Pool,
    id: &str,
    content: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE messages SET content = ?, has_attachments = 1 WHERE id = ?")
        .bind(content)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            eprintln!("[update_message_content_with_attachments] SQL error: {}", msg);
            msg
        })?;
    Ok(())
}

#[tauri::command]
pub async fn get_messages(pool: State<'_, Pool>, chat_id: String) -> Result<Vec<DbMessage>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments FROM messages WHERE chat_id = ? ORDER BY timestamp",
    )
    .bind(&chat_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[get_messages] SQL error: {}", e);
        e.to_string()
    })?;

    let messages = rows
        .into_iter()
        .map(|row| DbMessage {
            id: row.get("id"),
            chat_id: row.get("chat_id"),
            role: row.get("role"),
            content: row.get("content"),
            parent_id: row.get("parent_id"),
            timestamp: row.get("timestamp"),
            model: row.try_get("model").ok(),
            prompt_tokens: row.try_get("prompt_tokens").ok(),
            completion_tokens: row.try_get("completion_tokens").ok(),
            cost: row.try_get("cost").ok(),
            has_attachments: row.try_get("has_attachments").ok(),
        })
        .collect();
    Ok(messages)
}

#[tauri::command]
pub async fn delete_messages_after(
    app: AppHandle,
    pool: State<'_, Pool>,
    chat_id: String,
    timestamp: i64,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<(), String> {
    let ids_to_delete: Vec<String> = sqlx::query_scalar("SELECT id FROM messages WHERE chat_id = ? AND timestamp > ?")
        .bind(&chat_id)
        .bind(timestamp)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_messages_after] SQL error (fetch ids): {}", e);
            e.to_string()
        })?;
    for id in &ids_to_delete {
        if let Some(s) = store.as_ref().as_ref() {
            let _ = s.delete_by_message_id(id).await;
        }
        let _ = fts::fts_delete_message(pool.inner(), id).await;
        let _ = sqlx::query("UPDATE messages SET fts_indexed = 0 WHERE id = ?")
            .bind(id)
            .execute(pool.inner())
            .await;
    }
    let rows = sqlx::query(
        "SELECT id, content FROM messages WHERE chat_id = ? AND timestamp > ? AND has_attachments = 1",
    )
    .bind(&chat_id)
    .bind(timestamp)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[delete_messages_after] SQL error (fetch attachments): {}", e);
        e.to_string()
    })?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    sqlx::query("DELETE FROM messages WHERE chat_id = ? AND timestamp > ?")
        .bind(&chat_id)
        .bind(timestamp)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_messages_after] SQL error (delete): {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[derive(Deserialize)]
pub struct UpdateChatTitleArgs {
    #[serde(rename = "chatId")]
    chat_id: String,
    title: String,
}

#[tauri::command]
pub async fn update_chat_title(
    pool: State<'_, Pool>,
    args: UpdateChatTitleArgs,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query("UPDATE chats SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&args.title)
        .bind(now)
        .bind(&args.chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[update_chat_title] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_model(
    pool: State<'_, Pool>,
    chat_id: String,
    model: String,
    is_image_model: Option<bool>,
) -> Result<(), String> {
    let is_image = is_image_model.unwrap_or(false) as i64;
    sqlx::query("UPDATE chats SET model = ?, is_image_model = ?, updated_at = strftime('%s', 'now') WHERE id = ?")
        .bind(&model)
        .bind(is_image)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[update_chat_model] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}
