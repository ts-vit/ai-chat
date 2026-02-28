// CRUD для папок чатов
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::chat::DbFolder;

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn get_all_folders(pool: State<'_, Pool>) -> Result<Vec<DbFolder>, String> {
    let rows = sqlx::query(
        "SELECT id, name, color, sort_order, created_at FROM folders ORDER BY sort_order, name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[get_all_folders] SQL error: {}", e);
        e.to_string()
    })?;

    let folders = rows
        .into_iter()
        .map(|row| DbFolder {
            id: row.get("id"),
            name: row.get("name"),
            color: row.try_get("color").ok(),
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(folders)
}

#[tauri::command]
pub async fn create_folder(
    pool: State<'_, Pool>,
    name: String,
    color: Option<String>,
) -> Result<DbFolder, String> {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let sort_order: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM folders")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[create_folder] SQL error (max sort_order): {}", e);
            e.to_string()
        })?;

    sqlx::query(
        "INSERT INTO folders (id, name, color, sort_order, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&color)
    .bind(sort_order)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[create_folder] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbFolder {
        id: id.clone(),
        name,
        color,
        sort_order,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_folder(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    color: Option<String>,
) -> Result<(), String> {
    sqlx::query("UPDATE folders SET name = ?, color = ? WHERE id = ?")
        .bind(&name)
        .bind(&color)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[update_folder] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_folder(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    sqlx::query("UPDATE chats SET folder_id = NULL WHERE folder_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_folder] SQL error (nullify chats): {}", e);
            e.to_string()
        })?;
    sqlx::query("DELETE FROM folders WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[delete_folder] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReorderFoldersInput {
    pub folder_ids: Vec<String>,
}

#[tauri::command]
pub async fn reorder_folders(
    pool: State<'_, Pool>,
    input: ReorderFoldersInput,
) -> Result<(), String> {
    for (index, folder_id) in input.folder_ids.into_iter().enumerate() {
        let sort_order = index as i64;
        sqlx::query("UPDATE folders SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(&folder_id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                eprintln!("[reorder_folders] SQL error: {}", e);
                e.to_string()
            })?;
    }
    Ok(())
}

#[tauri::command]
pub async fn move_chat_to_folder(
    pool: State<'_, Pool>,
    chat_id: String,
    folder_id: Option<String>,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET folder_id = ? WHERE id = ?")
        .bind(&folder_id)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[move_chat_to_folder] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}
