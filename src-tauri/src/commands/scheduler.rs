use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;
use std::sync::Arc;

use crate::models::scheduler::{ScheduledTask, SchedulerStatus};
use crate::services::scheduler::{compute_next_run, SchedulerManager};

type Pool = sqlx::SqlitePool;

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

#[tauri::command]
pub async fn list_scheduled_tasks(
    pool: State<'_, Pool>,
) -> Result<Vec<ScheduledTask>, String> {
    let rows = sqlx::query("SELECT * FROM scheduled_tasks ORDER BY created_at DESC")
        .fetch_all(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows.iter().map(row_to_task).collect())
}

#[tauri::command]
pub async fn create_scheduled_task(
    pool: State<'_, Pool>,
    name: String,
    prompt: String,
    cron_expression: String,
    model: Option<String>,
    skill_id: Option<String>,
    project_id: Option<String>,
    deliver_telegram: bool,
    deliver_desktop_notification: bool,
) -> Result<ScheduledTask, String> {
    // Validate cron expression
    let next_run = compute_next_run(&cron_expression)
        .ok_or_else(|| "Invalid cron expression".to_string())?;

    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query(
        "INSERT INTO scheduled_tasks (id, name, prompt, cron_expression, enabled, mode, model, skill_id, project_id, deliver_telegram, deliver_desktop_notification, next_run_at, run_count, created_at, updated_at) VALUES (?, ?, ?, ?, 1, 'assistant', ?, ?, ?, ?, ?, ?, 0, ?, ?)"
    )
    .bind(&id)
    .bind(&name)
    .bind(&prompt)
    .bind(&cron_expression)
    .bind(&model)
    .bind(&skill_id)
    .bind(&project_id)
    .bind(deliver_telegram as i32)
    .bind(deliver_desktop_notification as i32)
    .bind(next_run)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(ScheduledTask {
        id,
        name,
        prompt,
        cron_expression,
        enabled: true,
        mode: "assistant".to_string(),
        model,
        skill_id,
        project_id,
        deliver_telegram,
        deliver_desktop_notification,
        last_run_at: None,
        next_run_at: Some(next_run),
        last_run_status: None,
        last_run_error: None,
        run_count: 0,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_scheduled_task(
    pool: State<'_, Pool>,
    id: String,
    name: Option<String>,
    prompt: Option<String>,
    cron_expression: Option<String>,
    enabled: Option<bool>,
    model: Option<String>,
    skill_id: Option<String>,
    project_id: Option<String>,
    deliver_telegram: Option<bool>,
    deliver_desktop_notification: Option<bool>,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    // Validate cron if provided
    if let Some(ref c) = cron_expression {
        compute_next_run(c).ok_or_else(|| "Invalid cron expression".to_string())?;
    }

    // Update each field individually (simple, Send-safe approach)
    if let Some(ref n) = name {
        let _ = sqlx::query("UPDATE scheduled_tasks SET name = ?, updated_at = ? WHERE id = ?")
            .bind(n).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if let Some(ref p) = prompt {
        let _ = sqlx::query("UPDATE scheduled_tasks SET prompt = ?, updated_at = ? WHERE id = ?")
            .bind(p).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if let Some(ref c) = cron_expression {
        let _ = sqlx::query("UPDATE scheduled_tasks SET cron_expression = ?, updated_at = ? WHERE id = ?")
            .bind(c).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if let Some(e) = enabled {
        let _ = sqlx::query("UPDATE scheduled_tasks SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(e as i32).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if let Some(dt) = deliver_telegram {
        let _ = sqlx::query("UPDATE scheduled_tasks SET deliver_telegram = ?, updated_at = ? WHERE id = ?")
            .bind(dt as i32).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if let Some(dn) = deliver_desktop_notification {
        let _ = sqlx::query("UPDATE scheduled_tasks SET deliver_desktop_notification = ?, updated_at = ? WHERE id = ?")
            .bind(dn as i32).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if model.is_some() {
        let _ = sqlx::query("UPDATE scheduled_tasks SET model = ?, updated_at = ? WHERE id = ?")
            .bind(&model).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if skill_id.is_some() {
        let _ = sqlx::query("UPDATE scheduled_tasks SET skill_id = ?, updated_at = ? WHERE id = ?")
            .bind(&skill_id).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }
    if project_id.is_some() {
        let _ = sqlx::query("UPDATE scheduled_tasks SET project_id = ?, updated_at = ? WHERE id = ?")
            .bind(&project_id).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }

    // Recompute next_run_at if cron or enabled changed
    if cron_expression.is_some() || enabled.is_some() {
        // Get current cron expression
        let cron_expr: String = sqlx::query_scalar(
            "SELECT cron_expression FROM scheduled_tasks WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

        let is_enabled: bool = sqlx::query_scalar::<_, i32>(
            "SELECT enabled FROM scheduled_tasks WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())? != 0;

        let next_run = if is_enabled { compute_next_run(&cron_expr) } else { None };
        let _ = sqlx::query("UPDATE scheduled_tasks SET next_run_at = ?, updated_at = ? WHERE id = ?")
            .bind(next_run).bind(now).bind(&id)
            .execute(pool.inner()).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_scheduled_task(
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    // Clear scheduled_task_id from associated chats
    let _ = sqlx::query("UPDATE chats SET scheduled_task_id = NULL WHERE scheduled_task_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    sqlx::query("DELETE FROM scheduled_tasks WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn toggle_scheduled_task(
    pool: State<'_, Pool>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let next_run = if enabled {
        let cron_expr: String = sqlx::query_scalar(
            "SELECT cron_expression FROM scheduled_tasks WHERE id = ?"
        )
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        compute_next_run(&cron_expr)
    } else {
        None
    };

    sqlx::query(
        "UPDATE scheduled_tasks SET enabled = ?, next_run_at = ?, updated_at = ? WHERE id = ?"
    )
    .bind(enabled as i32)
    .bind(next_run)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn run_scheduled_task_now(
    _app: AppHandle,
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    // Set next_run_at to now so the scheduler picks it up on next tick
    sqlx::query(
        "UPDATE scheduled_tasks SET next_run_at = ?, enabled = 1, updated_at = ? WHERE id = ?"
    )
    .bind(now)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_scheduler_status(
    pool: State<'_, Pool>,
    scheduler_state: State<'_, Arc<SchedulerManager>>,
) -> Result<SchedulerStatus, String> {
    let running = scheduler_state.is_running().await;

    let task_count: i32 = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM scheduled_tasks WHERE enabled = 1"
    )
    .fetch_one(pool.inner())
    .await
    .unwrap_or(0);

    let next_task_at: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(next_run_at) FROM scheduled_tasks WHERE enabled = 1 AND next_run_at IS NOT NULL"
    )
    .fetch_one(pool.inner())
    .await
    .unwrap_or(None);

    Ok(SchedulerStatus {
        running,
        task_count,
        next_task_at,
    })
}
