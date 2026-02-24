// Команды для работы с SQLite (чаты и сообщения)
use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::commands::attachments::delete_attachments_for_message;
use crate::models::chat::{DbChat, DbMessage};

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn create_chat(
    pool: State<'_, Pool>,
    title: String,
    system_prompt: String,
) -> Result<DbChat, String> {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, system_prompt) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(now)
    .bind(now)
    .bind(&system_prompt)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(DbChat {
        id: id.clone(),
        title,
        created_at: now,
        updated_at: now,
        system_prompt: Some(system_prompt).filter(|s| !s.is_empty()),
    })
}

#[tauri::command]
pub async fn delete_chat(app: AppHandle, pool: State<'_, Pool>, id: String) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, content FROM messages WHERE chat_id = ? AND has_attachments = 1")
        .bind(&id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    sqlx::query("DELETE FROM chats WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_chats(pool: State<'_, Pool>) -> Result<Vec<DbChat>, String> {
    let rows = sqlx::query(
        "SELECT id, title, created_at, updated_at, system_prompt FROM chats ORDER BY updated_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let chats = rows
        .into_iter()
        .map(|row| DbChat {
            id: row.get("id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            system_prompt: row.try_get::<String, _>("system_prompt").ok(),
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
    .map_err(|e| e.to_string())?;

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
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_message_content(
    pool: State<'_, Pool>,
    id: String,
    content: String,
) -> Result<(), String> {
    sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
        .bind(&content)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;
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
    .map_err(|e| e.to_string())?;

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
) -> Result<(), String> {
    let rows = sqlx::query(
        "SELECT id, content FROM messages WHERE chat_id = ? AND timestamp > ? AND has_attachments = 1",
    )
    .bind(&chat_id)
    .bind(timestamp)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    sqlx::query("DELETE FROM messages WHERE chat_id = ? AND timestamp > ?")
        .bind(&chat_id)
        .bind(timestamp)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_title(
    pool: State<'_, Pool>,
    id: String,
    title: String,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query("UPDATE chats SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&title)
        .bind(now)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
