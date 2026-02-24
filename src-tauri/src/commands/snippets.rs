// Команды для категорий и шаблонов промптов (сниппеты)
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::chat::{DbCategory, DbSnippet};

type Pool = sqlx::SqlitePool;

fn unix_now() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .map(|d| d.as_secs() as i64)
}

#[tauri::command]
pub async fn create_category(pool: State<'_, Pool>, name: String) -> Result<DbCategory, String> {
    let id = Uuid::new_v4().to_string();
    let now = unix_now()?;
    sqlx::query("INSERT INTO categories (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&name)
        .bind(now)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(DbCategory {
        id: id.clone(),
        name,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_category(
    pool: State<'_, Pool>,
    id: String,
    name: String,
) -> Result<(), String> {
    sqlx::query("UPDATE categories SET name = ? WHERE id = ?")
        .bind(&name)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_category(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    let row = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM snippets WHERE category_id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    if row > 0 {
        return Err("Нельзя удалить категорию, к ней привязаны сниппеты".to_string());
    }
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_categories(pool: State<'_, Pool>) -> Result<Vec<DbCategory>, String> {
    let rows = sqlx::query("SELECT id, name, created_at FROM categories ORDER BY name")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    let list = rows
        .into_iter()
        .map(|row| DbCategory {
            id: row.get("id"),
            name: row.get("name"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(list)
}

#[tauri::command]
pub async fn create_snippet(
    pool: State<'_, Pool>,
    name: String,
    content: String,
    category_id: String,
) -> Result<DbSnippet, String> {
    let id = Uuid::new_v4().to_string();
    let now = unix_now()?;
    sqlx::query(
        "INSERT INTO snippets (id, name, content, category_id, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&content)
    .bind(&category_id)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(DbSnippet {
        id: id.clone(),
        name,
        content,
        category_id,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_snippet(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    content: String,
    category_id: String,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE snippets SET name = ?, content = ?, category_id = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&content)
    .bind(&category_id)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_snippet(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM snippets WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_snippets(pool: State<'_, Pool>) -> Result<Vec<DbSnippet>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, category_id, created_at FROM snippets ORDER BY name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    let list = rows
        .into_iter()
        .map(|row| DbSnippet {
            id: row.get("id"),
            name: row.get("name"),
            content: row.get("content"),
            category_id: row.get("category_id"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(list)
}

#[tauri::command]
pub async fn get_snippets_by_category(
    pool: State<'_, Pool>,
    category_id: String,
) -> Result<Vec<DbSnippet>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, category_id, created_at FROM snippets WHERE category_id = ? ORDER BY name",
    )
    .bind(&category_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    let list = rows
        .into_iter()
        .map(|row| DbSnippet {
            id: row.get("id"),
            name: row.get("name"),
            content: row.get("content"),
            category_id: row.get("category_id"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(list)
}
