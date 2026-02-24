// Команды для работы с пресетами системных промптов
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::chat::DbPreset;

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn create_preset(
    pool: State<'_, Pool>,
    name: String,
    content: String,
    is_default: bool,
) -> Result<DbPreset, String> {
    if is_default {
        let _ = sqlx::query("UPDATE presets SET is_default = 0")
            .execute(pool.inner())
            .await;
    }

    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;
    let is_default_int = if is_default { 1i64 } else { 0i64 };

    sqlx::query(
        "INSERT INTO presets (id, name, content, is_default, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&content)
    .bind(is_default_int)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(DbPreset {
        id: id.clone(),
        name,
        content,
        is_default,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_preset(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    content: String,
    is_default: bool,
) -> Result<(), String> {
    if is_default {
        let _ = sqlx::query("UPDATE presets SET is_default = 0")
            .execute(pool.inner())
            .await;
    }

    let is_default_int = if is_default { 1i64 } else { 0i64 };

    sqlx::query(
        "UPDATE presets SET name = ?, content = ?, is_default = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&content)
    .bind(is_default_int)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_preset(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM presets WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_presets(pool: State<'_, Pool>) -> Result<Vec<DbPreset>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, is_default, created_at FROM presets ORDER BY created_at",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let presets = rows
        .into_iter()
        .map(|row| DbPreset {
            id: row.get("id"),
            name: row.get("name"),
            content: row.get("content"),
            is_default: row.get::<i64, _>("is_default") != 0,
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(presets)
}

#[tauri::command]
pub async fn set_chat_system_prompt(
    pool: State<'_, Pool>,
    chat_id: String,
    system_prompt: String,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET system_prompt = ? WHERE id = ?")
        .bind(&system_prompt)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_chat_system_prompt(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<String, String> {
    let row = sqlx::query("SELECT system_prompt FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(row
        .map(|r| r.try_get::<String, _>("system_prompt").unwrap_or_default())
        .unwrap_or_default())
}
