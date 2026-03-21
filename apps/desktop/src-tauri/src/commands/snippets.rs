// Команды для категорий и шаблонов промптов (сниппеты)
use sqlx::Row;
use tauri::State;

use crate::models::db::{DbCategory, DbSnippet};

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn create_category(pool: State<'_, Pool>, name: String) -> Result<DbCategory, String> {
    let id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();
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
    show_on_welcome: Option<bool>,
) -> Result<DbSnippet, String> {
    let id = uni_common::generate_id();
    let now = uni_common::now_unix_secs();
    let welcome_val = show_on_welcome.unwrap_or(false) as i32;
    sqlx::query(
        "INSERT INTO snippets (id, name, content, category_id, created_at, show_on_welcome) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&content)
    .bind(&category_id)
    .bind(now)
    .bind(welcome_val)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(DbSnippet {
        id: id.clone(),
        name,
        content,
        category_id,
        created_at: now,
        show_on_welcome: Some(welcome_val),
    })
}

#[tauri::command]
pub async fn update_snippet(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    content: String,
    category_id: String,
    show_on_welcome: Option<bool>,
) -> Result<(), String> {
    let welcome_val = show_on_welcome.unwrap_or(false) as i32;
    sqlx::query(
        "UPDATE snippets SET name = ?, content = ?, category_id = ?, show_on_welcome = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&content)
    .bind(&category_id)
    .bind(welcome_val)
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

fn map_snippet_row(row: sqlx::sqlite::SqliteRow) -> DbSnippet {
    DbSnippet {
        id: row.get("id"),
        name: row.get("name"),
        content: row.get("content"),
        category_id: row.get("category_id"),
        created_at: row.get("created_at"),
        show_on_welcome: row.get("show_on_welcome"),
    }
}

#[tauri::command]
pub async fn get_all_snippets(pool: State<'_, Pool>) -> Result<Vec<DbSnippet>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, category_id, created_at, show_on_welcome FROM snippets ORDER BY name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_snippet_row).collect())
}

#[tauri::command]
pub async fn get_snippets_by_category(
    pool: State<'_, Pool>,
    category_id: String,
) -> Result<Vec<DbSnippet>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, category_id, created_at, show_on_welcome FROM snippets WHERE category_id = ? ORDER BY name",
    )
    .bind(&category_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_snippet_row).collect())
}

#[tauri::command]
pub async fn get_welcome_snippets(pool: State<'_, Pool>) -> Result<Vec<DbSnippet>, String> {
    let rows = sqlx::query(
        "SELECT id, name, content, category_id, created_at, show_on_welcome
         FROM snippets
         ORDER BY CASE WHEN show_on_welcome = 1 THEN 0 ELSE 1 END, created_at DESC
         LIMIT 4",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().map(map_snippet_row).collect())
}
