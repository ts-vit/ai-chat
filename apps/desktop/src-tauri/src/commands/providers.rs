// CRUD для кастомных провайдеров (Custom)
use sqlx::Row;
use tauri::State;

use crate::models::db::{CreateCustomProviderInput, DbCustomProvider};

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn get_custom_providers(pool: State<'_, Pool>) -> Result<Vec<DbCustomProvider>, String> {
    let rows = sqlx::query(
        "SELECT id, name, base_url, api_key, created_at FROM custom_providers ORDER BY created_at ASC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let list = rows
        .into_iter()
        .map(|row| DbCustomProvider {
            id: row.get("id"),
            name: row.get("name"),
            base_url: row.get("base_url"),
            api_key: row.get("api_key"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(list)
}

#[tauri::command]
pub async fn create_custom_provider(
    pool: State<'_, Pool>,
    input: CreateCustomProviderInput,
) -> Result<DbCustomProvider, String> {
    let id = uni_common::generate_id();
    let created_at = uni_common::now_unix_secs();

    sqlx::query(
        "INSERT INTO custom_providers (id, name, base_url, api_key, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(created_at)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(DbCustomProvider {
        id: id.clone(),
        name: input.name,
        base_url: input.base_url,
        api_key: input.api_key,
        created_at,
    })
}

#[tauri::command]
pub async fn update_custom_provider(
    pool: State<'_, Pool>,
    id: String,
    input: CreateCustomProviderInput,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE custom_providers SET name = ?, base_url = ?, api_key = ? WHERE id = ?",
    )
    .bind(&input.name)
    .bind(&input.base_url)
    .bind(&input.api_key)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_provider_chat_count(pool: State<'_, Pool>, provider_id: String) -> Result<i64, String> {
    let row = sqlx::query("SELECT COUNT(*) as cnt FROM chats WHERE provider_id = ?")
        .bind(&provider_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(row.get::<i64, _>("cnt"))
}

#[tauri::command]
pub async fn delete_custom_provider(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    sqlx::query("UPDATE chats SET provider_id = 'openrouter', model = '' WHERE provider_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM custom_providers WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}
