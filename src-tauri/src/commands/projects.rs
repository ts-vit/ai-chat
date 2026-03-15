// CRUD для проектов (Assistant mode)
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::project::{Project, ProjectSummary};

type Pool = sqlx::SqlitePool;

fn now_secs() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .map(|d| d.as_secs() as i64)
}

#[tauri::command]
pub async fn create_project(
    pool: State<'_, Pool>,
    name: String,
    goal: String,
) -> Result<Project, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_secs()?;

    sqlx::query(
        "INSERT INTO projects (id, name, goal, status, created_at, updated_at) VALUES (?, ?, ?, 'active', ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&goal)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[create_project] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(Project {
        id,
        name,
        goal,
        status: "active".to_string(),
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn get_project(
    pool: State<'_, Pool>,
    id: String,
) -> Result<ProjectSummary, String> {
    let row = sqlx::query(
        "SELECT p.*,
            (SELECT COUNT(*) FROM chats WHERE project_id = p.id) as chat_count,
            (SELECT COUNT(*) FROM workspace_artifacts WHERE project_id = p.id) as artifact_count,
            (SELECT COUNT(*) FROM agent_memory WHERE project_id = p.id) as memory_count,
            (SELECT COUNT(*) FROM agent_plans WHERE project_id = p.id) as plan_count
        FROM projects p WHERE p.id = ?",
    )
    .bind(&id)
    .fetch_one(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_project] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(ProjectSummary {
        id: row.get("id"),
        name: row.get("name"),
        goal: row.try_get("goal").unwrap_or_default(),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        chat_count: row.get("chat_count"),
        artifact_count: row.get("artifact_count"),
        memory_count: row.get("memory_count"),
        plan_count: row.get("plan_count"),
    })
}

#[tauri::command]
pub async fn list_projects(
    pool: State<'_, Pool>,
    status_filter: Option<String>,
) -> Result<Vec<ProjectSummary>, String> {
    let sql = match &status_filter {
        Some(_) => "SELECT p.*,
            (SELECT COUNT(*) FROM chats WHERE project_id = p.id) as chat_count,
            (SELECT COUNT(*) FROM workspace_artifacts WHERE project_id = p.id) as artifact_count,
            (SELECT COUNT(*) FROM agent_memory WHERE project_id = p.id) as memory_count,
            (SELECT COUNT(*) FROM agent_plans WHERE project_id = p.id) as plan_count
        FROM projects p WHERE p.status = ? ORDER BY p.updated_at DESC",
        None => "SELECT p.*,
            (SELECT COUNT(*) FROM chats WHERE project_id = p.id) as chat_count,
            (SELECT COUNT(*) FROM workspace_artifacts WHERE project_id = p.id) as artifact_count,
            (SELECT COUNT(*) FROM agent_memory WHERE project_id = p.id) as memory_count,
            (SELECT COUNT(*) FROM agent_plans WHERE project_id = p.id) as plan_count
        FROM projects p ORDER BY p.updated_at DESC",
    };

    let rows = if let Some(ref status) = status_filter {
        sqlx::query(sql).bind(status).fetch_all(pool.inner()).await
    } else {
        sqlx::query(sql).fetch_all(pool.inner()).await
    }
    .map_err(|e| {
        log::error!("[list_projects] SQL error: {}", e);
        e.to_string()
    })?;

    let projects = rows
        .into_iter()
        .map(|row| ProjectSummary {
            id: row.get("id"),
            name: row.get("name"),
            goal: row.try_get("goal").unwrap_or_default(),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            chat_count: row.get("chat_count"),
            artifact_count: row.get("artifact_count"),
            memory_count: row.get("memory_count"),
            plan_count: row.get("plan_count"),
        })
        .collect();
    Ok(projects)
}

#[tauri::command]
pub async fn update_project(
    pool: State<'_, Pool>,
    id: String,
    name: Option<String>,
    goal: Option<String>,
    status: Option<String>,
) -> Result<Project, String> {
    let now = now_secs()?;

    // Build dynamic UPDATE
    let mut sets = vec!["updated_at = ?"];
    let mut binds: Vec<String> = vec![now.to_string()];

    if let Some(ref n) = name {
        sets.push("name = ?");
        binds.push(n.clone());
    }
    if let Some(ref g) = goal {
        sets.push("goal = ?");
        binds.push(g.clone());
    }
    if let Some(ref s) = status {
        sets.push("status = ?");
        binds.push(s.clone());
    }

    let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));
    let mut query = sqlx::query(&sql);
    for b in &binds {
        query = query.bind(b);
    }
    query = query.bind(&id);
    query.execute(pool.inner()).await.map_err(|e| {
        log::error!("[update_project] SQL error: {}", e);
        e.to_string()
    })?;

    // Fetch updated row
    let row = sqlx::query("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_project] fetch error: {}", e);
            e.to_string()
        })?;

    Ok(Project {
        id: row.get("id"),
        name: row.get("name"),
        goal: row.try_get("goal").unwrap_or_default(),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn delete_project(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    // 1. Free chats
    sqlx::query("UPDATE chats SET project_id = NULL WHERE project_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_project] nullify chats: {}", e);
            e.to_string()
        })?;

    // 2. Delete project-level workspace artifacts
    sqlx::query("DELETE FROM workspace_artifacts WHERE project_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_project] delete artifacts: {}", e);
            e.to_string()
        })?;

    // 3. Delete project-level agent memory
    sqlx::query("DELETE FROM agent_memory WHERE project_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_project] delete memory: {}", e);
            e.to_string()
        })?;

    // 4. Delete project-level agent plans
    sqlx::query("DELETE FROM agent_plans WHERE project_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_project] delete plans: {}", e);
            e.to_string()
        })?;

    // 5. Delete project
    sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_project] SQL error: {}", e);
            e.to_string()
        })?;

    Ok(())
}

#[tauri::command]
pub async fn archive_project(pool: State<'_, Pool>, id: String) -> Result<Project, String> {
    let now = now_secs()?;

    sqlx::query("UPDATE projects SET status = 'archived', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[archive_project] SQL error: {}", e);
            e.to_string()
        })?;

    let row = sqlx::query("SELECT * FROM projects WHERE id = ?")
        .bind(&id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[archive_project] fetch error: {}", e);
            e.to_string()
        })?;

    Ok(Project {
        id: row.get("id"),
        name: row.get("name"),
        goal: row.try_get("goal").unwrap_or_default(),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn assign_chat_to_project(
    pool: State<'_, Pool>,
    chat_id: String,
    project_id: String,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET project_id = ? WHERE id = ?")
        .bind(&project_id)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[assign_chat_to_project] SQL error: {}", e);
            e.to_string()
        })?;

    // Update project's updated_at
    let now = now_secs()?;
    let _ = sqlx::query("UPDATE projects SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&project_id)
        .execute(pool.inner())
        .await;

    Ok(())
}

#[tauri::command]
pub async fn remove_chat_from_project(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET project_id = NULL WHERE id = ?")
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[remove_chat_from_project] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}
