use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::workspace::WorkspaceArtifact;

type Pool = sqlx::SqlitePool;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn row_to_artifact(row: &sqlx::sqlite::SqliteRow) -> WorkspaceArtifact {
    WorkspaceArtifact {
        id: row.get("id"),
        chat_id: row.get("chat_id"),
        project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
        name: row.get("name"),
        content_type: row.try_get::<String, _>("content_type").unwrap_or_else(|_| "text".to_string()),
        content: row.try_get::<Option<String>, _>("content").ok().flatten(),
        file_path: row.try_get::<Option<String>, _>("file_path").ok().flatten(),
        created_by: row.try_get::<Option<String>, _>("created_by").ok().flatten(),
        updated_by: row.try_get::<Option<String>, _>("updated_by").ok().flatten(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

// --- Public helper functions (used by agent loop) ---

pub async fn create_artifact_impl(
    pool: &Pool,
    chat_id: &str,
    project_id: Option<&str>,
    name: &str,
    content_type: &str,
    content: Option<&str>,
    created_by: Option<&str>,
) -> Result<WorkspaceArtifact, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_secs();

    sqlx::query(
        "INSERT INTO workspace_artifacts (id, chat_id, project_id, name, content_type, content, created_by, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(chat_id)
    .bind(project_id)
    .bind(name)
    .bind(content_type)
    .bind(content)
    .bind(created_by)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| {
        log::error!("[create_artifact_impl] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(WorkspaceArtifact {
        id,
        chat_id: chat_id.to_string(),
        project_id: project_id.map(|s| s.to_string()),
        name: name.to_string(),
        content_type: content_type.to_string(),
        content: content.map(|s| s.to_string()),
        file_path: None,
        created_by: created_by.map(|s| s.to_string()),
        updated_by: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn read_artifact_impl(
    pool: &Pool,
    chat_id: &str,
    project_id: Option<&str>,
    name: &str,
) -> Result<Option<WorkspaceArtifact>, String> {
    let cols = "id, chat_id, project_id, name, content_type, content, file_path, created_by, updated_by, created_at, updated_at";
    let row = if let Some(pid) = project_id {
        sqlx::query(&format!(
            "SELECT {} FROM workspace_artifacts WHERE project_id = ? AND name = ? LIMIT 1", cols
        ))
        .bind(pid)
        .bind(name)
        .fetch_optional(pool)
        .await
    } else {
        sqlx::query(&format!(
            "SELECT {} FROM workspace_artifacts WHERE chat_id = ? AND project_id IS NULL AND name = ? LIMIT 1", cols
        ))
        .bind(chat_id)
        .bind(name)
        .fetch_optional(pool)
        .await
    }
    .map_err(|e| {
        log::error!("[read_artifact_impl] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(row.as_ref().map(row_to_artifact))
}

pub async fn list_artifacts_impl(
    pool: &Pool,
    chat_id: &str,
    project_id: Option<&str>,
) -> Result<Vec<WorkspaceArtifact>, String> {
    let cols = "id, chat_id, project_id, name, content_type, content, file_path, created_by, updated_by, created_at, updated_at";
    let rows = if let Some(pid) = project_id {
        sqlx::query(&format!(
            "SELECT {} FROM workspace_artifacts WHERE project_id = ? ORDER BY updated_at DESC", cols
        ))
        .bind(pid)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(&format!(
            "SELECT {} FROM workspace_artifacts WHERE chat_id = ? AND project_id IS NULL ORDER BY updated_at DESC", cols
        ))
        .bind(chat_id)
        .fetch_all(pool)
        .await
    }
    .map_err(|e| {
        log::error!("[list_artifacts_impl] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(rows.iter().map(row_to_artifact).collect())
}

pub async fn update_artifact_impl(
    pool: &Pool,
    chat_id: &str,
    project_id: Option<&str>,
    name: &str,
    content: &str,
    updated_by: Option<&str>,
) -> Result<(), String> {
    let now = now_secs();
    if let Some(pid) = project_id {
        sqlx::query(
            "UPDATE workspace_artifacts SET content = ?, updated_by = ?, updated_at = ? WHERE project_id = ? AND name = ?"
        )
        .bind(content)
        .bind(updated_by)
        .bind(now)
        .bind(pid)
        .bind(name)
        .execute(pool)
        .await
    } else {
        sqlx::query(
            "UPDATE workspace_artifacts SET content = ?, updated_by = ?, updated_at = ? WHERE chat_id = ? AND project_id IS NULL AND name = ?"
        )
        .bind(content)
        .bind(updated_by)
        .bind(now)
        .bind(chat_id)
        .bind(name)
        .execute(pool)
        .await
    }
    .map_err(|e| {
        log::error!("[update_artifact_impl] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

// --- Tauri commands ---

#[tauri::command]
pub async fn list_workspace_artifacts(
    pool: State<'_, Pool>,
    chat_id: String,
    project_id: Option<String>,
) -> Result<Vec<WorkspaceArtifact>, String> {
    list_artifacts_impl(pool.inner(), &chat_id, project_id.as_deref()).await
}

#[tauri::command]
pub async fn get_workspace_artifact(
    pool: State<'_, Pool>,
    id: String,
) -> Result<Option<WorkspaceArtifact>, String> {
    let row = sqlx::query(
        "SELECT id, chat_id, project_id, name, content_type, content, file_path, created_by, updated_by, created_at, updated_at FROM workspace_artifacts WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_workspace_artifact] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(row.as_ref().map(row_to_artifact))
}

#[tauri::command]
pub async fn create_workspace_artifact(
    pool: State<'_, Pool>,
    chat_id: String,
    project_id: Option<String>,
    name: String,
    content_type: String,
    content: Option<String>,
) -> Result<WorkspaceArtifact, String> {
    create_artifact_impl(pool.inner(), &chat_id, project_id.as_deref(), &name, &content_type, content.as_deref(), None).await
}

#[tauri::command]
pub async fn update_workspace_artifact(
    pool: State<'_, Pool>,
    id: String,
    name: Option<String>,
    content: Option<String>,
) -> Result<(), String> {
    let now = now_secs();
    if let Some(new_name) = &name {
        sqlx::query("UPDATE workspace_artifacts SET name = ?, updated_at = ? WHERE id = ?")
            .bind(new_name)
            .bind(now)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(new_content) = &content {
        sqlx::query("UPDATE workspace_artifacts SET content = ?, updated_at = ? WHERE id = ?")
            .bind(new_content)
            .bind(now)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_workspace_artifact(
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM workspace_artifacts WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_workspace_artifact] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_chat_workspace(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM workspace_artifacts WHERE chat_id = ?")
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_chat_workspace] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}
