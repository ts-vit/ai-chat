use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use sqlx::Row;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::AgentCancelTokens;
use crate::models::plan::{AgentPlan, AgentPlanWithProgress, AgentTask, PlanWithTasks};
use crate::commands::budget::check_budget;
use crate::services::http_client::build_http_client;
use crate::services::mcp_manager::McpManager;
use crate::services::memory_vector_store::MemoryVectorStore;

type Pool = sqlx::SqlitePool;

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ─── LLM plan generation system prompt ──────────────────────────

const PLAN_SYSTEM_PROMPT: &str = r#"You are a task planner. Given a goal, decompose it into a list of concrete, actionable subtasks.

Return ONLY a valid JSON array with this schema, no markdown, no explanation:
[
  {
    "title": "Short task title",
    "description": "Detailed description of what needs to be done",
    "dependencies": [],
    "category": "coding" | "writing" | "analysis" | "general" | "fast" | "vision"
  }
]

Category must be one of: coding, writing, analysis, general, fast, vision. Use "coding" for code-related tasks, "writing" for text/content, "analysis" for reasoning/research, "vision" for image-related, "fast" for simple/quick tasks, "general" otherwise.

Rules:
- Each task should be independently executable by an AI agent with tool access
- Tasks should be ordered logically (earlier tasks first)
- Use dependency indices to express ordering constraints (e.g. [0, 1] means this task depends on tasks at index 0 and 1)
- Keep tasks granular but not trivial — each should take 1-5 agent iterations
- Typically 3-10 tasks for most goals
- Dependencies reference 0-based array indices, not IDs"#;

// ─── Resolve catalog id to (model_id, base_url, api_key) for execution ───

pub async fn resolve_catalog_id_to_credentials(
    pool: &Pool,
    app: &AppHandle,
    catalog_id: &str,
) -> Result<(String, Option<String>, String), String> {
    use tauri_plugin_store::StoreExt;
    let row = sqlx::query("SELECT provider, model_id FROM model_catalog WHERE id = ?")
        .bind(catalog_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Model not found in catalog: {}", catalog_id))?;
    let provider: String = row.get("provider");
    let model_id: String = row.get("model_id");

    let store = app.store("settings.json").map_err(|e: tauri_plugin_store::Error| e.to_string())?;
    match provider.as_str() {
        "openrouter" => {
            let api_key = store.get("api_key").and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
            Ok((model_id, None, api_key))
        }
        "ollama" => {
            let base_url = store.get("ollamaUrl").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "http://localhost:11434/v1".to_string());
            Ok((model_id, Some(base_url), String::new()))
        }
        "custom" => {
            let rest = catalog_id.strip_prefix("custom:").unwrap_or(catalog_id);
            let provider_id = rest.splitn(2, ':').next().unwrap_or(rest);
            let row = sqlx::query("SELECT base_url, api_key FROM custom_providers WHERE id = ?")
                .bind(provider_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Custom provider not found: {}", provider_id))?;
            let base_url: String = row.get("base_url");
            let api_key: String = row.get("api_key");
            Ok((model_id, Some(base_url), api_key))
        }
        _ => Err(format!("Unknown provider in catalog: {}", provider)),
    }
}

// ─── DB helpers ─────────────────────────────────────────────────

async fn get_plan_from_db(pool: &Pool, plan_id: &str) -> Result<AgentPlan, String> {
    let row = sqlx::query(
        "SELECT id, chat_id, project_id, goal, status, execution_mode, replan_count, created_at, updated_at FROM agent_plans WHERE id = ?"
    )
    .bind(plan_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Plan not found: {}", plan_id))?;

    Ok(AgentPlan {
        id: row.get("id"),
        chat_id: row.get("chat_id"),
        project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
        goal: row.get("goal"),
        status: row.get("status"),
        execution_mode: row.get("execution_mode"),
        replan_count: row.try_get("replan_count").unwrap_or(0),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn get_tasks_for_plan(pool: &Pool, plan_id: &str) -> Result<Vec<AgentTask>, String> {
    let rows = sqlx::query(
        "SELECT id, plan_id, title, description, status, dependencies, result, agent_run_id, sort_order, created_at, updated_at, category, assigned_model FROM agent_tasks WHERE plan_id = ? ORDER BY sort_order"
    )
    .bind(plan_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut tasks = Vec::new();
    for row in rows {
        let deps_json: String = row.get("dependencies");
        let dependencies: Vec<String> = serde_json::from_str(&deps_json).unwrap_or_default();
        tasks.push(AgentTask {
            id: row.get("id"),
            plan_id: row.get("plan_id"),
            title: row.get("title"),
            description: row.get("description"),
            status: row.get("status"),
            dependencies,
            result: row.get("result"),
            agent_run_id: row.get("agent_run_id"),
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            category: row.try_get("category").ok(),
            assigned_model: row.try_get("assigned_model").ok(),
        });
    }
    Ok(tasks)
}

/// Truncate a string to at most `max_bytes` bytes without splitting a UTF-8 char.
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Strip markdown code fences from LLM response
fn strip_markdown_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

// ─── Commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_plan(
    app: AppHandle,
    pool: State<'_, Pool>,
    chat_id: String,
    goal: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
) -> Result<PlanWithTasks, String> {
    let client = build_http_client(&app, Some(Duration::from_secs(120))).await?;

    let url = base_url
        .as_deref()
        .unwrap_or("https://openrouter.ai/api/v1");
    let url = format!("{}/chat/completions", url.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key).parse().unwrap(),
        );
    }
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": PLAN_SYSTEM_PROMPT },
            { "role": "user", "content": goal }
        ],
        "stream": false
    });

    // Only include temperature/max_tokens for models that support them
    if !crate::commands::chat::is_image_generation_model(&model) {
        let m = model.to_lowercase();
        let is_reasoning = m.contains("o1") || m.contains("o3") || m.contains("o4");
        if !is_reasoning {
            body["temperature"] = serde_json::json!(0.2);
            body["max_tokens"] = serde_json::json!(4000);
        }
    }

    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Plan generation request failed: {}", e))?;

    let status = response.status();
    let resp_text = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("LLM API error ({}): {}", status, resp_text));
    }

    let resp_json: serde_json::Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Invalid JSON response: {}", e))?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in LLM response")?;

    let cleaned = strip_markdown_fences(content);

    #[derive(serde::Deserialize)]
    struct RawTask {
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        dependencies: Vec<usize>,
        #[serde(default)]
        category: Option<String>,
    }

    let raw_tasks: Vec<RawTask> =
        serde_json::from_str(cleaned).map_err(|e| format!("Failed to parse plan JSON: {}. Content: {}", e, cleaned))?;

    if raw_tasks.is_empty() {
        return Err("LLM generated an empty plan".to_string());
    }

    // Generate IDs
    let plan_id = Uuid::new_v4().to_string();
    let now = unix_now();

    let task_ids: Vec<String> = raw_tasks.iter().map(|_| Uuid::new_v4().to_string()).collect();

    // Convert dependency indices to UUIDs; normalize category
    let mut tasks: Vec<AgentTask> = Vec::new();
    for (i, raw) in raw_tasks.iter().enumerate() {
        let dep_uuids: Vec<String> = raw
            .dependencies
            .iter()
            .filter_map(|&idx| task_ids.get(idx).cloned())
            .collect();
        let category = raw
            .category
            .as_deref()
            .map(|c| c.to_lowercase())
            .filter(|c| matches!(c.as_str(), "coding" | "writing" | "analysis" | "general" | "fast" | "vision"))
            .unwrap_or_else(|| "general".to_string());

        tasks.push(AgentTask {
            id: task_ids[i].clone(),
            plan_id: plan_id.clone(),
            title: raw.title.clone(),
            description: raw.description.clone(),
            status: "pending".to_string(),
            dependencies: dep_uuids,
            result: None,
            agent_run_id: None,
            sort_order: i as i32,
            created_at: now,
            updated_at: now,
            category: Some(category),
            assigned_model: None,
        });
    }

    // Read project_id from chat
    let project_id: Option<String> = sqlx::query_scalar(
        "SELECT project_id FROM chats WHERE id = ?"
    )
    .bind(&chat_id)
    .fetch_optional(pool.inner())
    .await
    .unwrap_or(None);

    // Save plan to DB
    sqlx::query(
        "INSERT INTO agent_plans (id, chat_id, project_id, goal, status, execution_mode, created_at, updated_at) VALUES (?, ?, ?, ?, 'draft', 'manual', ?, ?)"
    )
    .bind(&plan_id)
    .bind(&chat_id)
    .bind(&project_id)
    .bind(&goal)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    // Save tasks to DB
    for task in &tasks {
        let deps_json = serde_json::to_string(&task.dependencies).unwrap_or_else(|_| "[]".to_string());
        let cat = task.category.as_deref().unwrap_or("general");
        sqlx::query(
            "INSERT INTO agent_tasks (id, plan_id, title, description, status, dependencies, result, agent_run_id, sort_order, created_at, updated_at, category, assigned_model) VALUES (?, ?, ?, ?, 'pending', ?, NULL, NULL, ?, ?, ?, ?, NULL)"
        )
        .bind(&task.id)
        .bind(&plan_id)
        .bind(&task.title)
        .bind(&task.description)
        .bind(&deps_json)
        .bind(task.sort_order)
        .bind(now)
        .bind(now)
        .bind(cat)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }

    // Routing: assign model per task if enabled (use chat's model/base_url/api_key for LLM routing call)
    let store = tauri_plugin_store::StoreExt::store(&app, "settings.json").map_err(|e: tauri_plugin_store::Error| e.to_string())?;
    let routing_enabled: bool = store.get("routingEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let routing_strategy: String = store.get("routingStrategy").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "rules".to_string());

    let base_url_str = base_url.as_deref().unwrap_or("https://openrouter.ai/api/v1");

    if routing_enabled {
        for task in &tasks {
            let assigned = if routing_strategy == "llm" {
                let task_desc = format!("{}: {}", task.title, task.description);
                crate::commands::routing::llm_route_task(
                    pool.inner(),
                    &app,
                    &chat_id,
                    &api_key,
                    base_url_str,
                    &model,
                    &task_desc,
                ).await.ok().flatten()
            } else {
                let cat = task.category.as_deref().unwrap_or("general");
                crate::commands::routing::get_model_for_category(pool.inner(), cat).await
            };
            if let Some(assigned) = assigned {
                let _ = sqlx::query("UPDATE agent_tasks SET assigned_model = ? WHERE id = ?")
                    .bind(&assigned)
                    .bind(&task.id)
                    .execute(pool.inner())
                    .await;
            }
        }
    }

    let plan = AgentPlan {
        id: plan_id.clone(),
        chat_id: chat_id.clone(),
        project_id: project_id.clone(),
        goal: goal.clone(),
        status: "draft".to_string(),
        execution_mode: "manual".to_string(),
        replan_count: 0,
        created_at: now,
        updated_at: now,
    };

    // Reload tasks from DB so result includes assigned_model set by routing
    let tasks_with_models = get_tasks_for_plan(pool.inner(), &plan_id).await?;
    let result = PlanWithTasks {
        plan: plan.clone(),
        tasks: tasks_with_models,
    };

    let _ = app.emit("plan-created", serde_json::json!({
        "planId": plan_id,
        "chatId": chat_id,
        "goal": goal,
        "tasks": tasks,
    }));

    Ok(result)
}

#[tauri::command]
pub async fn get_plan(
    pool: State<'_, Pool>,
    plan_id: String,
) -> Result<PlanWithTasks, String> {
    let plan = get_plan_from_db(pool.inner(), &plan_id).await?;
    let tasks = get_tasks_for_plan(pool.inner(), &plan_id).await?;
    Ok(PlanWithTasks { plan, tasks })
}

#[tauri::command]
pub async fn get_plans_for_chat(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<Vec<AgentPlan>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, project_id, goal, status, execution_mode, replan_count, created_at, updated_at FROM agent_plans WHERE chat_id = ? ORDER BY created_at DESC"
    )
    .bind(&chat_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let plans = rows
        .iter()
        .map(|row| AgentPlan {
            id: row.get("id"),
            chat_id: row.get("chat_id"),
            project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
            goal: row.get("goal"),
            status: row.get("status"),
            execution_mode: row.get("execution_mode"),
            replan_count: row.try_get("replan_count").unwrap_or(0),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(plans)
}

#[tauri::command]
pub async fn get_plans_for_project(
    pool: State<'_, Pool>,
    project_id: String,
) -> Result<Vec<AgentPlanWithProgress>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, project_id, goal, status, execution_mode, replan_count, created_at, updated_at FROM agent_plans WHERE project_id = ? ORDER BY created_at DESC"
    )
    .bind(&project_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut plans = Vec::new();
    for row in &rows {
        let plan = AgentPlan {
            id: row.get("id"),
            chat_id: row.get("chat_id"),
            project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
            goal: row.get("goal"),
            status: row.get("status"),
            execution_mode: row.get("execution_mode"),
            replan_count: row.try_get("replan_count").unwrap_or(0),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        let task_row = sqlx::query(
            "SELECT COUNT(*) as total, COALESCE(SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END), 0) as completed FROM agent_tasks WHERE plan_id = ?"
        )
        .bind(&plan.id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

        plans.push(AgentPlanWithProgress {
            plan,
            total_tasks: task_row.get("total"),
            completed_tasks: task_row.get("completed"),
        });
    }

    Ok(plans)
}

#[tauri::command]
pub async fn update_task(
    pool: State<'_, Pool>,
    task_id: String,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    assigned_model: Option<String>,
) -> Result<AgentTask, String> {
    let now = unix_now();

    if let Some(ref t) = title {
        sqlx::query("UPDATE agent_tasks SET title = ?, updated_at = ? WHERE id = ?")
            .bind(t)
            .bind(now)
            .bind(&task_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(ref d) = description {
        sqlx::query("UPDATE agent_tasks SET description = ?, updated_at = ? WHERE id = ?")
            .bind(d)
            .bind(now)
            .bind(&task_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    if let Some(ref s) = status {
        let valid = ["pending", "running", "completed", "failed", "skipped"];
        if !valid.contains(&s.as_str()) {
            return Err(format!("Invalid task status: {}", s));
        }
        sqlx::query("UPDATE agent_tasks SET status = ?, updated_at = ? WHERE id = ?")
            .bind(s)
            .bind(now)
            .bind(&task_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }
    if assigned_model.is_some() {
        sqlx::query("UPDATE agent_tasks SET assigned_model = ?, updated_at = ? WHERE id = ?")
            .bind(&assigned_model)
            .bind(now)
            .bind(&task_id)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
    }

    // Fetch and return updated task
    let row = sqlx::query(
        "SELECT id, plan_id, title, description, status, dependencies, result, agent_run_id, sort_order, created_at, updated_at, category, assigned_model FROM agent_tasks WHERE id = ?"
    )
    .bind(&task_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Task not found: {}", task_id))?;

    let deps_json: String = row.get("dependencies");
    let dependencies: Vec<String> = serde_json::from_str(&deps_json).unwrap_or_default();

    Ok(AgentTask {
        id: row.get("id"),
        plan_id: row.get("plan_id"),
        title: row.get("title"),
        description: row.get("description"),
        status: row.get("status"),
        dependencies,
        result: row.get("result"),
        agent_run_id: row.get("agent_run_id"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        category: row.try_get("category").ok(),
        assigned_model: row.try_get("assigned_model").ok(),
    })
}

#[tauri::command]
pub async fn delete_plan(
    pool: State<'_, Pool>,
    plan_id: String,
) -> Result<(), String> {
    // Tasks cascade-delete via FK
    sqlx::query("DELETE FROM agent_tasks WHERE plan_id = ?")
        .bind(&plan_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM agent_plans WHERE id = ?")
        .bind(&plan_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn approve_plan(
    pool: State<'_, Pool>,
    plan_id: String,
    execution_mode: String,
) -> Result<AgentPlan, String> {
    if execution_mode != "auto" && execution_mode != "manual" {
        return Err(format!("Invalid execution mode: {}. Must be 'auto' or 'manual'", execution_mode));
    }

    let now = unix_now();

    let result = sqlx::query(
        "UPDATE agent_plans SET status = 'approved', execution_mode = ?, updated_at = ? WHERE id = ? AND status = 'draft'"
    )
    .bind(&execution_mode)
    .bind(now)
    .bind(&plan_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Plan not found or not in draft status".to_string());
    }

    get_plan_from_db(pool.inner(), &plan_id).await
}

#[tauri::command]
pub async fn start_plan_execution(
    app: AppHandle,
    pool: State<'_, Pool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    kb_vector_store: State<'_, Arc<Option<crate::services::kb_vector_store::KbVectorStore>>>,
    cancel_tokens: State<'_, AgentCancelTokens>,
    plan_id: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    supports_tool_use: Option<bool>,
) -> Result<(), String> {
    let now = unix_now();

    // Update plan status to running
    let result = sqlx::query(
        "UPDATE agent_plans SET status = 'running', updated_at = ? WHERE id = ? AND status IN ('draft', 'approved', 'paused')"
    )
    .bind(now)
    .bind(&plan_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Plan not found or already running/completed".to_string());
    }

    let plan = get_plan_from_db(pool.inner(), &plan_id).await?;
    let tasks = get_tasks_for_plan(pool.inner(), &plan_id).await?;

    let _ = app.emit("plan-status-changed", serde_json::json!({
        "planId": plan_id,
        "status": "running"
    }));

    // Find first ready task
    let completed_ids: HashSet<String> = tasks
        .iter()
        .filter(|t| t.status == "completed")
        .map(|t| t.id.clone())
        .collect();

    let next_task = tasks.iter().find(|t| {
        t.status == "pending"
            && t.dependencies.iter().all(|d| completed_ids.contains(d))
    });

    let task = match next_task {
        Some(t) => t,
        None => {
            // No ready tasks — check if all done
            let all_done = tasks.iter().all(|t| t.status == "completed" || t.status == "skipped");
            let final_status = if all_done { "completed" } else { "failed" };
            let _ = sqlx::query("UPDATE agent_plans SET status = ?, updated_at = ? WHERE id = ?")
                .bind(final_status)
                .bind(now)
                .bind(&plan_id)
                .execute(pool.inner())
                .await;
            let _ = app.emit("plan-status-changed", serde_json::json!({
                "planId": plan_id,
                "status": final_status
            }));
            return Ok(());
        }
    };

    // Build enriched system prompt
    let enriched_prompt = build_task_system_prompt(
        system_prompt.as_deref(),
        &plan,
        task,
        &tasks,
    );

    // Prepare agent run
    let (run_id, user_msg_id, cancel_token) =
        prepare_task_agent_run(&app, pool.inner(), cancel_tokens.inner(), &plan.chat_id, task).await?;

    // Update task status
    let _ = sqlx::query(
        "UPDATE agent_tasks SET status = 'running', agent_run_id = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&run_id)
    .bind(now)
    .bind(&task.id)
    .execute(pool.inner())
    .await;

    let _ = app.emit("plan-task-started", serde_json::json!({
        "planId": plan_id,
        "taskId": task.id,
        "agentRunId": run_id
    }));

    if let Err(e) = check_budget(pool.inner(), &app, &plan.chat_id, Some(&plan_id)).await {
        return Err(e);
    }

    let (run_model, run_base_url, run_api_key) = if let Some(ref assigned) = task.assigned_model {
        resolve_catalog_id_to_credentials(pool.inner(), &app, assigned).await
            .unwrap_or_else(|_| (model.clone(), base_url.clone(), api_key.clone()))
    } else {
        (model, base_url, api_key)
    };

    // Spawn agent loop with auto-advance loop
    spawn_plan_task(
        app,
        pool.inner().clone(),
        mcp_manager.inner().clone(),
        memory_store.inner().clone(),
        kb_vector_store.inner().clone(),
        cancel_tokens.inner().clone(),
        plan.chat_id.clone(),
        run_id,
        user_msg_id,
        cancel_token,
        task.id.clone(),
        plan_id,
        run_model,
        run_base_url,
        run_api_key,
        enriched_prompt,
        temperature,
        max_tokens,
        top_p,
        top_k,
        frequency_penalty,
        presence_penalty,
        supports_tool_use,
    );

    Ok(())
}

#[tauri::command]
pub async fn execute_single_task(
    app: AppHandle,
    pool: State<'_, Pool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    memory_store: State<'_, Arc<Option<MemoryVectorStore>>>,
    kb_vector_store: State<'_, Arc<Option<crate::services::kb_vector_store::KbVectorStore>>>,
    cancel_tokens: State<'_, AgentCancelTokens>,
    task_id: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    supports_tool_use: Option<bool>,
) -> Result<String, String> {
    let now = unix_now();

    // Get task
    let row = sqlx::query(
        "SELECT id, plan_id, title, description, status, dependencies, result, agent_run_id, sort_order, created_at, updated_at, category, assigned_model FROM agent_tasks WHERE id = ?"
    )
    .bind(&task_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| format!("Task not found: {}", task_id))?;

    let deps_json: String = row.get("dependencies");
    let dependencies: Vec<String> = serde_json::from_str(&deps_json).unwrap_or_default();
    let task = AgentTask {
        id: row.get("id"),
        plan_id: row.get("plan_id"),
        title: row.get("title"),
        description: row.get("description"),
        status: row.get("status"),
        dependencies,
        result: row.get("result"),
        agent_run_id: row.get("agent_run_id"),
        sort_order: row.get("sort_order"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        category: row.try_get("category").ok(),
        assigned_model: row.try_get("assigned_model").ok(),
    };

    if task.status != "pending" && task.status != "failed" {
        return Err(format!("Task is not in pending/failed status: {}", task.status));
    }

    // Validate dependencies
    let plan = get_plan_from_db(pool.inner(), &task.plan_id).await?;
    let all_tasks = get_tasks_for_plan(pool.inner(), &task.plan_id).await?;

    let completed_ids: HashSet<String> = all_tasks
        .iter()
        .filter(|t| t.status == "completed")
        .map(|t| t.id.clone())
        .collect();

    let unmet: Vec<&String> = task
        .dependencies
        .iter()
        .filter(|d| !completed_ids.contains(*d))
        .collect();

    if !unmet.is_empty() {
        return Err(format!("Unmet dependencies: {:?}", unmet));
    }

    // Ensure plan is running
    if plan.status != "running" {
        let _ = sqlx::query(
            "UPDATE agent_plans SET status = 'running', updated_at = ? WHERE id = ?"
        )
        .bind(now)
        .bind(&plan.id)
        .execute(pool.inner())
        .await;

        let _ = app.emit("plan-status-changed", serde_json::json!({
            "planId": plan.id,
            "status": "running"
        }));
    }

    // Build enriched system prompt
    let enriched_prompt = build_task_system_prompt(
        system_prompt.as_deref(),
        &plan,
        &task,
        &all_tasks,
    );

    // Prepare agent run
    let (run_id, user_msg_id, cancel_token) =
        prepare_task_agent_run(&app, pool.inner(), cancel_tokens.inner(), &plan.chat_id, &task).await?;

    // Update task
    let _ = sqlx::query(
        "UPDATE agent_tasks SET status = 'running', agent_run_id = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&run_id)
    .bind(now)
    .bind(&task_id)
    .execute(pool.inner())
    .await;

    let _ = app.emit("plan-task-started", serde_json::json!({
        "planId": task.plan_id,
        "taskId": task_id,
        "agentRunId": run_id
    }));

    let ret_run_id = run_id.clone();

    if let Err(e) = check_budget(pool.inner(), &app, &plan.chat_id, Some(&task.plan_id)).await {
        return Err(e);
    }

    let (run_model, run_base_url, run_api_key) = if let Some(ref assigned) = task.assigned_model {
        resolve_catalog_id_to_credentials(pool.inner(), &app, assigned).await
            .unwrap_or_else(|_| (model.clone(), base_url.clone(), api_key.clone()))
    } else {
        (model, base_url, api_key)
    };

    // Spawn agent loop (with auto-advance for auto mode)
    spawn_plan_task(
        app,
        pool.inner().clone(),
        mcp_manager.inner().clone(),
        memory_store.inner().clone(),
        kb_vector_store.inner().clone(),
        cancel_tokens.inner().clone(),
        plan.chat_id.clone(),
        run_id,
        user_msg_id,
        cancel_token,
        task.id.clone(),
        task.plan_id.clone(),
        run_model,
        run_base_url,
        run_api_key,
        enriched_prompt,
        temperature,
        max_tokens,
        top_p,
        top_k,
        frequency_penalty,
        presence_penalty,
        supports_tool_use,
    );

    Ok(ret_run_id)
}

// ─── Replan on failure ──────────────────────────────────────────

const REPLAN_SYSTEM_PROMPT: &str = r#"You are a task planner. A plan is being executed and a task has failed. Revise the remaining tasks to work around the failure.

Return ONLY a valid JSON array of REMAINING tasks (do not include completed tasks).
Same schema as before:
[
  {
    "title": "Short task title",
    "description": "Detailed description of what needs to be done",
    "dependencies": []
  }
]

Rules:
- Dependencies reference 0-based indices within THIS new array only
- You may modify, remove, or add tasks
- Do not include already-completed tasks
- Try to achieve the original goal despite the failure
- If the goal is impossible after this failure, return an empty array []"#;

async fn replan(
    app: &AppHandle,
    pool: &Pool,
    plan: &AgentPlan,
    tasks: &[AgentTask],
    failed_task: &AgentTask,
    error_info: &str,
    model: &str,
    api_key: &str,
    base_url: &Option<String>,
) -> Result<Vec<AgentTask>, String> {
    let client = build_http_client(app, Some(Duration::from_secs(120))).await?;

    let url = base_url
        .as_deref()
        .unwrap_or("https://openrouter.ai/api/v1");
    let url = format!("{}/chat/completions", url.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        headers.insert(
            AUTHORIZATION,
            format!("Bearer {}", api_key).parse().unwrap(),
        );
    }
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    // Build context about current task statuses
    let mut status_lines = String::new();
    for t in tasks {
        let result_info = if let Some(ref r) = t.result {
            format!(" → {}", truncate_str(r, 200))
        } else {
            String::new()
        };
        status_lines.push_str(&format!("- [{}] {}: {}{}\n", t.status, t.title, t.description, result_info));
    }

    let user_msg = format!(
        "Goal: {}\n\nCurrent task statuses:\n{}\nFailed task: {}\nError: {}\n\nRevise the remaining plan to work around this failure.",
        plan.goal, status_lines, failed_task.title, error_info
    );

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": REPLAN_SYSTEM_PROMPT },
            { "role": "user", "content": user_msg }
        ],
        "stream": false
    });

    let m = model.to_lowercase();
    let is_reasoning = m.contains("o1") || m.contains("o3") || m.contains("o4");
    if !is_reasoning {
        body["temperature"] = serde_json::json!(0.2);
        body["max_tokens"] = serde_json::json!(4000);
    }

    log::info!("[planner] Calling LLM for replan of plan {}", plan.id);

    let response = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Replan request failed: {}", e))?;

    let status = response.status();
    let resp_text = response.text().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("LLM API error ({}): {}", status, resp_text));
    }

    let resp_json: serde_json::Value =
        serde_json::from_str(&resp_text).map_err(|e| format!("Invalid JSON response: {}", e))?;

    let content = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("No content in replan response")?;

    let cleaned = strip_markdown_fences(content);

    #[derive(serde::Deserialize)]
    struct RawTask {
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        dependencies: Vec<usize>,
    }

    let raw_tasks: Vec<RawTask> =
        serde_json::from_str(cleaned).map_err(|e| format!("Failed to parse replan JSON: {}. Content: {}", e, cleaned))?;

    if raw_tasks.is_empty() {
        return Ok(Vec::new());
    }

    let now = unix_now();
    let task_ids: Vec<String> = raw_tasks.iter().map(|_| Uuid::new_v4().to_string()).collect();

    let mut new_tasks: Vec<AgentTask> = Vec::new();
    for (i, raw) in raw_tasks.iter().enumerate() {
        let dep_uuids: Vec<String> = raw
            .dependencies
            .iter()
            .filter_map(|&idx| task_ids.get(idx).cloned())
            .collect();

        new_tasks.push(AgentTask {
            id: task_ids[i].clone(),
            plan_id: plan.id.clone(),
            title: raw.title.clone(),
            description: raw.description.clone(),
            status: "pending".to_string(),
            dependencies: dep_uuids,
            result: None,
            agent_run_id: None,
            sort_order: i as i32,
            created_at: now,
            updated_at: now,
            category: None,
            assigned_model: None,
        });
    }

    Ok(new_tasks)
}

// ─── Internal helpers ───────────────────────────────────────────

fn build_task_system_prompt(
    original: Option<&str>,
    plan: &AgentPlan,
    task: &AgentTask,
    all_tasks: &[AgentTask],
) -> String {
    let total = all_tasks.len();
    let task_num = task.sort_order + 1;

    let mut prompt = String::new();

    if let Some(orig) = original {
        if !orig.is_empty() {
            prompt.push_str(orig);
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str(&format!(
        "## Current Task\nYou are executing task {}/{} of a plan to achieve: {}\n\n**Task:** {}\n**Description:** {}\n",
        task_num, total, plan.goal, task.title, task.description
    ));

    // Add context from completed dependencies
    let dep_results: Vec<(&AgentTask, &str)> = task
        .dependencies
        .iter()
        .filter_map(|dep_id| {
            all_tasks.iter().find(|t| &t.id == dep_id).and_then(|t| {
                t.result.as_deref().map(|r| (t, r))
            })
        })
        .collect();

    if !dep_results.is_empty() {
        prompt.push_str("\n## Context from previous tasks:\n");
        for (dep_task, result) in dep_results {
            // Truncate long results
            let truncated = truncate_str(result, 2000);
            prompt.push_str(&format!("- Task \"{}\": {}\n", dep_task.title, truncated));
        }
    }

    prompt.push_str("\nFocus on completing THIS specific task. When done, provide a clear summary of what was accomplished.");

    prompt
}

/// Prepare an agent run for a plan task: create DB records, user message, cancel token.
/// Returns (run_id, user_msg_id, cancel_token) for spawning agent_loop.
async fn prepare_task_agent_run(
    app: &AppHandle,
    pool: &Pool,
    cancel_tokens: &AgentCancelTokens,
    chat_id: &str,
    task: &AgentTask,
) -> Result<(String, String, tokio_util::sync::CancellationToken), String> {
    let run_id = Uuid::new_v4().to_string();
    let now = unix_now();
    let max_iter: i64 = 25;

    // Create agent_run record
    sqlx::query(
        "INSERT INTO agent_runs (id, chat_id, status, iterations, max_iterations, started_at) VALUES (?, ?, 'running', 0, ?, ?)"
    )
    .bind(&run_id)
    .bind(chat_id)
    .bind(max_iter)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Create user message with task description
    let user_msg_id = Uuid::new_v4().to_string();
    let content = format!("Execute task: {}\n\n{}", task.title, task.description);

    // Find last message in chat for parent_id
    let parent_id: Option<String> = sqlx::query_scalar(
        "SELECT id FROM messages WHERE chat_id = ? ORDER BY timestamp DESC LIMIT 1"
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, agent_step, agent_run_id) VALUES (?, ?, 'user', ?, ?, ?, '', 0, 0, 0.0, 0, 0, ?)"
    )
    .bind(&user_msg_id)
    .bind(chat_id)
    .bind(&content)
    .bind(&parent_id)
    .bind(now)
    .bind(&run_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Update active_child_map
    if let Some(ref pid) = parent_id {
        let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
            .bind(chat_id)
            .fetch_one(pool)
            .await
            .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|_| "{}".to_string());
        let mut map: std::collections::HashMap<String, String> =
            serde_json::from_str(&map_str).unwrap_or_default();
        map.insert(pid.clone(), user_msg_id.clone());
        let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
        let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
            .bind(&new_map)
            .bind(chat_id)
            .execute(pool)
            .await;
    }

    // Emit agent-run-started
    let _ = app.emit("agent-run-started", serde_json::json!({
        "runId": run_id,
        "chatId": chat_id,
        "maxIterations": max_iter,
    }));

    // Create and store cancel token
    let cancel_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = cancel_tokens.write().await;
        tokens.insert(run_id.clone(), cancel_token.clone());
    }

    Ok((run_id, user_msg_id, cancel_token))
}

/// Start an agent run for a plan task. Spawns agent_loop and handles
/// auto-advancement in a loop (no recursion).
fn spawn_plan_task(
    app: AppHandle,
    pool: Pool,
    mcp_manager: Arc<McpManager>,
    memory_store: Arc<Option<MemoryVectorStore>>,
    kb_store: Arc<Option<crate::services::kb_vector_store::KbVectorStore>>,
    cancel_tokens: AgentCancelTokens,
    chat_id: String,
    run_id: String,
    user_msg_id: String,
    cancel_token: tokio_util::sync::CancellationToken,
    task_id: String,
    plan_id: String,
    model: String,
    base_url: Option<String>,
    api_key: String,
    system_prompt: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    supports_tool_use: Option<bool>,
) {
    let max_iter: i64 = 25;
    tokio::spawn(async move {
        let assistant_msg_id = Uuid::new_v4().to_string();

        // Run agent loop for first task
        crate::commands::agent::agent_loop(
            app.clone(),
            pool.clone(),
            mcp_manager.clone(),
            memory_store.clone(),
            kb_store.clone(),
            chat_id.clone(),
            run_id.clone(),
            model.clone(),
            base_url.clone(),
            api_key.clone(),
            Some(system_prompt),
            user_msg_id,
            assistant_msg_id,
            max_iter,
            true,
            cancel_token,
            cancel_tokens.clone(),
            temperature,
            max_tokens,
            top_p,
            top_k,
            frequency_penalty,
            presence_penalty,
            supports_tool_use,
            None, // scope_agent_run_id — sequential, no scoping needed
            Some(plan_id.clone()),
            0, // depth
        )
        .await;

        // Update task status based on agent_run outcome
        let mut current_run_id = run_id;
        let mut current_task_id = task_id;
        let mut skip_status_update = false; // true after parallel batch (statuses already updated)

        loop {
            let now = unix_now();

            // After parallel execution, statuses are already updated — skip to ready-task search
            let task_status = if skip_status_update {
                skip_status_update = false;
                // Determine if we came from a failed parallel task
                if !current_run_id.is_empty() {
                    let s: String = sqlx::query_scalar("SELECT status FROM agent_runs WHERE id = ?")
                        .bind(&current_run_id)
                        .fetch_optional(&pool)
                        .await
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "failed".to_string());
                    if s == "completed" { "completed" } else { "failed" }
                } else {
                    "completed" // all parallel tasks succeeded
                }
            } else {
                // Read agent_run status (sequential task just finished)
                let run_status: String = match sqlx::query_scalar(
                    "SELECT status FROM agent_runs WHERE id = ?"
                )
                .bind(&current_run_id)
                .fetch_optional(&pool)
                .await
                {
                    Ok(Some(s)) => s,
                    _ => break,
                };

                let task_status = if run_status == "completed" { "completed" } else { "failed" };

                // Get task result
                let result = if task_status == "completed" {
                    sqlx::query_scalar::<_, String>(
                        "SELECT content FROM messages WHERE agent_run_id = ? AND role = 'assistant' ORDER BY timestamp DESC LIMIT 1"
                    )
                    .bind(&current_run_id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                } else {
                    None
                };

                let _ = sqlx::query(
                    "UPDATE agent_tasks SET status = ?, result = ?, updated_at = ? WHERE id = ?"
                )
                .bind(task_status)
                .bind(&result)
                .bind(now)
                .bind(&current_task_id)
                .execute(&pool)
                .await;

                let _ = app.emit("plan-task-updated", serde_json::json!({
                    "planId": plan_id,
                    "taskId": current_task_id,
                    "status": task_status
                }));

                task_status
            };

            // Check if auto-advance is needed
            let plan = match get_plan_from_db(&pool, &plan_id).await {
                Ok(p) => p,
                Err(_) => break,
            };

            if plan.status != "running" || plan.execution_mode != "auto" {
                break;
            }

            if task_status == "failed" {
                // Attempt replan if under limit
                if plan.replan_count < 2 {
                    let error_info: String = sqlx::query_scalar(
                        "SELECT COALESCE(error, 'Unknown error') FROM agent_runs WHERE id = ?"
                    )
                    .bind(&current_run_id)
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "Unknown error".to_string());

                    let tasks_snapshot = match get_tasks_for_plan(&pool, &plan_id).await {
                        Ok(t) => t,
                        Err(_) => break,
                    };
                    let failed_task = tasks_snapshot.iter().find(|t| t.id == current_task_id);

                    if let Some(ft) = failed_task {
                        match replan(&app, &pool, &plan, &tasks_snapshot, ft, &error_info, &model, &api_key, &base_url).await {
                            Ok(new_tasks) if !new_tasks.is_empty() => {
                                // Delete pending tasks
                                let _ = sqlx::query(
                                    "DELETE FROM agent_tasks WHERE plan_id = ? AND status = 'pending'"
                                )
                                .bind(&plan_id)
                                .execute(&pool)
                                .await;

                                // Insert new tasks
                                let base_order = tasks_snapshot.len() as i32;
                                for (i, task) in new_tasks.iter().enumerate() {
                                    let deps_json = serde_json::to_string(&task.dependencies).unwrap_or_else(|_| "[]".to_string());
                                    let _ = sqlx::query(
                                        "INSERT INTO agent_tasks (id, plan_id, title, description, status, dependencies, result, agent_run_id, sort_order, created_at, updated_at) VALUES (?, ?, ?, ?, 'pending', ?, NULL, NULL, ?, ?, ?)"
                                    )
                                    .bind(&task.id)
                                    .bind(&plan_id)
                                    .bind(&task.title)
                                    .bind(&task.description)
                                    .bind(&deps_json)
                                    .bind(base_order + i as i32)
                                    .bind(now)
                                    .bind(now)
                                    .execute(&pool)
                                    .await;
                                }

                                // Increment replan count
                                let _ = sqlx::query(
                                    "UPDATE agent_plans SET replan_count = replan_count + 1, updated_at = ? WHERE id = ?"
                                )
                                .bind(now)
                                .bind(&plan_id)
                                .execute(&pool)
                                .await;

                                let _ = app.emit("plan-replanned", serde_json::json!({
                                    "planId": plan_id,
                                    "newTaskCount": new_tasks.len()
                                }));

                                log::info!("[planner] Replanned: {} new tasks for plan {}", new_tasks.len(), plan_id);

                                // Reset current tracking so loop finds next ready task via parallel/sequential path
                                // We use a dummy run_id/task_id — the loop will skip status update and go to ready-task search
                                // Actually, just continue the outer loop — we need to find next ready tasks
                                // The trick: set current_run_id to empty so the next iteration's status read fails gracefully
                                // Better approach: jump directly to the ready-task search below
                                // We'll use a goto-like pattern by NOT breaking and letting execution fall through
                            }
                            _ => {
                                // Replan failed or returned empty — fail the plan
                                log::warn!("[planner] Replan returned empty or failed for plan {}", plan_id);
                            }
                        }
                    }
                }

                // Check if replan happened (replan_count changed)
                let updated_plan = match get_plan_from_db(&pool, &plan_id).await {
                    Ok(p) => p,
                    Err(_) => break,
                };
                if updated_plan.replan_count == plan.replan_count {
                    // No replan happened — fail the plan
                    let _ = sqlx::query(
                        "UPDATE agent_plans SET status = 'failed', updated_at = ? WHERE id = ?"
                    )
                    .bind(now)
                    .bind(&plan_id)
                    .execute(&pool)
                    .await;

                    let _ = app.emit("plan-status-changed", serde_json::json!({
                        "planId": plan_id,
                        "status": "failed"
                    }));
                    break;
                }
                // Replan succeeded — fall through to find next ready tasks
            }

            // Find ALL ready tasks (parallel execution support)
            let tasks = match get_tasks_for_plan(&pool, &plan_id).await {
                Ok(t) => t,
                Err(_) => break,
            };

            let done_ids: HashSet<String> = tasks
                .iter()
                .filter(|t| t.status == "completed" || t.status == "skipped")
                .map(|t| t.id.clone())
                .collect();

            let ready_tasks: Vec<&AgentTask> = tasks.iter().filter(|t| {
                t.status == "pending"
                    && t.dependencies.iter().all(|d| done_ids.contains(d))
            }).collect();

            if ready_tasks.is_empty() {
                // No more ready tasks — finalize plan
                let all_done = tasks
                    .iter()
                    .all(|t| t.status == "completed" || t.status == "skipped");

                if all_done {
                    let _ = sqlx::query(
                        "UPDATE agent_plans SET status = 'completed', updated_at = ? WHERE id = ?"
                    )
                    .bind(now)
                    .bind(&plan_id)
                    .execute(&pool)
                    .await;

                    let _ = app.emit("plan-status-changed", serde_json::json!({
                        "planId": plan_id,
                        "status": "completed"
                    }));
                }
                break;
            }

            // Read chat settings (shared for all tasks in this batch)
            let chat_row = match sqlx::query(
                "SELECT system_prompt, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty FROM chats WHERE id = ?"
            )
            .bind(&plan.chat_id)
            .fetch_optional(&pool)
            .await
            {
                Ok(Some(r)) => r,
                _ => break,
            };

            let next_sys_prompt: Option<String> = chat_row.get("system_prompt");
            let next_temp: Option<f32> = chat_row.get("temperature");
            let next_max_tok: Option<u32> = chat_row.get("max_tokens");
            let next_top_p: Option<f32> = chat_row.get("top_p");
            let next_top_k: Option<u32> = chat_row.get("top_k");
            let next_freq: Option<f32> = chat_row.get("frequency_penalty");
            let next_pres: Option<f32> = chat_row.get("presence_penalty");

            // Re-fetch tasks for enriched prompts
            let fresh_tasks = match get_tasks_for_plan(&pool, &plan_id).await {
                Ok(t) => t,
                Err(_) => break,
            };

            // Reload plan for replan_count
            let plan = match get_plan_from_db(&pool, &plan_id).await {
                Ok(p) => p,
                Err(_) => break,
            };

            if ready_tasks.len() == 1 {
                // Single task — run sequentially (original behavior, no message scoping needed)
                let task = ready_tasks[0];

                let enriched_prompt = build_task_system_prompt(
                    next_sys_prompt.as_deref(),
                    &plan,
                    task,
                    &fresh_tasks,
                );

                let (next_run_id, next_user_msg_id, next_cancel_token) =
                    match prepare_task_agent_run(&app, &pool, &cancel_tokens, &plan.chat_id, task).await {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("[planner] Failed to prepare next task: {}", e);
                            break;
                        }
                    };

                let _ = sqlx::query(
                    "UPDATE agent_tasks SET status = 'running', agent_run_id = ?, updated_at = ? WHERE id = ?"
                )
                .bind(&next_run_id)
                .bind(now)
                .bind(&task.id)
                .execute(&pool)
                .await;

                let _ = app.emit("plan-task-started", serde_json::json!({
                    "planId": plan_id,
                    "taskId": task.id,
                    "agentRunId": next_run_id
                }));

                let next_assistant_msg_id = Uuid::new_v4().to_string();
                crate::commands::agent::agent_loop(
                    app.clone(),
                    pool.clone(),
                    mcp_manager.clone(),
                    memory_store.clone(),
                    kb_store.clone(),
                    plan.chat_id.clone(),
                    next_run_id.clone(),
                    model.clone(),
                    base_url.clone(),
                    api_key.clone(),
                    Some(enriched_prompt),
                    next_user_msg_id,
                    next_assistant_msg_id,
                    max_iter,
                    true,
                    next_cancel_token,
                    cancel_tokens.clone(),
                    next_temp,
                    next_max_tok,
                    next_top_p,
                    next_top_k,
                    next_freq,
                    next_pres,
                    Some(true),
                    None, // scope_agent_run_id — sequential, no scoping needed
                    Some(plan_id.clone()),
                    0, // depth
                )
                .await;

                current_run_id = next_run_id;
                current_task_id = task.id.clone();
            } else {
                // Multiple ready tasks — run in PARALLEL with message isolation
                log::info!("[planner] Running {} tasks in parallel for plan {}", ready_tasks.len(), plan_id);

                let mut handles = Vec::new();

                for task in &ready_tasks {
                    let enriched_prompt = build_task_system_prompt(
                        next_sys_prompt.as_deref(),
                        &plan,
                        task,
                        &fresh_tasks,
                    );

                    let (par_run_id, par_user_msg_id, par_cancel_token) =
                        match prepare_task_agent_run(&app, &pool, &cancel_tokens, &plan.chat_id, task).await {
                            Ok(v) => v,
                            Err(e) => {
                                log::error!("[planner] Failed to prepare parallel task {}: {}", task.id, e);
                                continue;
                            }
                        };

                    let _ = sqlx::query(
                        "UPDATE agent_tasks SET status = 'running', agent_run_id = ?, updated_at = ? WHERE id = ?"
                    )
                    .bind(&par_run_id)
                    .bind(now)
                    .bind(&task.id)
                    .execute(&pool)
                    .await;

                    let _ = app.emit("plan-task-started", serde_json::json!({
                        "planId": plan_id,
                        "taskId": task.id,
                        "agentRunId": par_run_id
                    }));

                    // Clone everything for the spawned task
                    let h_app = app.clone();
                    let h_pool = pool.clone();
                    let h_mcp = mcp_manager.clone();
                    let h_mem = memory_store.clone();
                    let h_kb = kb_store.clone();
                    let h_chat_id = plan.chat_id.clone();
                    let h_model = model.clone();
                    let h_base_url = base_url.clone();
                    let h_api_key = api_key.clone();
                    let h_cancel_tokens = cancel_tokens.clone();
                    let h_task_id = task.id.clone();
                    let h_run_id = par_run_id.clone();
                    let h_plan_id = plan_id.clone();

                    let handle = tokio::spawn(async move {
                        let assistant_msg_id = Uuid::new_v4().to_string();
                        crate::commands::agent::agent_loop(
                            h_app,
                            h_pool,
                            h_mcp,
                            h_mem,
                            h_kb,
                            h_chat_id,
                            h_run_id.clone(),
                            h_model,
                            h_base_url,
                            h_api_key,
                            Some(enriched_prompt),
                            par_user_msg_id,
                            assistant_msg_id,
                            25, // max_iter
                            true,
                            par_cancel_token,
                            h_cancel_tokens,
                            next_temp,
                            next_max_tok,
                            next_top_p,
                            next_top_k,
                            next_freq,
                            next_pres,
                            Some(true),
                            Some(h_run_id.clone()), // scope_agent_run_id — parallel isolation
                            Some(h_plan_id),
                            0, // depth
                        )
                        .await;
                        (h_run_id, h_task_id)
                    });

                    handles.push(handle);
                }

                // Wait for all parallel tasks to complete
                let results = futures::future::join_all(handles).await;

                // Process results — update each task's status
                let mut any_failed = false;
                let mut last_run_id = String::new();
                let mut last_task_id = String::new();

                for result in results {
                    match result {
                        Ok((par_run_id, par_task_id)) => {
                            let run_status: String = sqlx::query_scalar(
                                "SELECT status FROM agent_runs WHERE id = ?"
                            )
                            .bind(&par_run_id)
                            .fetch_optional(&pool)
                            .await
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| "failed".to_string());

                            let par_task_status = if run_status == "completed" { "completed" } else { "failed" };

                            let par_result = if par_task_status == "completed" {
                                sqlx::query_scalar::<_, String>(
                                    "SELECT content FROM messages WHERE agent_run_id = ? AND role = 'assistant' ORDER BY timestamp DESC LIMIT 1"
                                )
                                .bind(&par_run_id)
                                .fetch_optional(&pool)
                                .await
                                .ok()
                                .flatten()
                            } else {
                                None
                            };

                            let _ = sqlx::query(
                                "UPDATE agent_tasks SET status = ?, result = ?, updated_at = ? WHERE id = ?"
                            )
                            .bind(par_task_status)
                            .bind(&par_result)
                            .bind(now)
                            .bind(&par_task_id)
                            .execute(&pool)
                            .await;

                            let _ = app.emit("plan-task-updated", serde_json::json!({
                                "planId": plan_id,
                                "taskId": par_task_id,
                                "status": par_task_status
                            }));

                            if par_task_status == "failed" {
                                any_failed = true;
                                last_run_id = par_run_id;
                                last_task_id = par_task_id;
                            }
                        }
                        Err(e) => {
                            log::error!("[planner] Parallel task panicked: {}", e);
                            any_failed = true;
                        }
                    }
                }

                if any_failed {
                    // Set to failed task so next loop iteration handles replan
                    current_run_id = last_run_id;
                    current_task_id = last_task_id;
                    skip_status_update = true;
                    continue;
                }

                // All parallel tasks succeeded — continue loop to find next batch
                current_run_id = String::new();
                current_task_id = String::new();
                skip_status_update = true;
                continue;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_unchanged() {
        assert_eq!(truncate_str("Hello world", 100), "Hello world");
    }

    #[test]
    fn test_truncate_exact_limit() {
        let s = "Hello";
        assert_eq!(truncate_str(s, 5), "Hello");
    }

    #[test]
    fn test_truncate_long_text() {
        let s = "Hello world, this is a long text";
        let result = truncate_str(s, 11);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_truncate_russian_utf8_safe() {
        // "Привет" = 12 bytes (6 Cyrillic chars × 2 bytes each)
        let s = "Привет мир";
        // Truncate at 13 bytes — should land on char boundary (after "Привет" + space)
        let result = truncate_str(s, 13);
        assert!(result.len() <= 13);
        assert!(s.is_char_boundary(result.len()));
        // Truncate at 14 — in the middle of 'м' (byte 13-14) → should go back to 13
        let result2 = truncate_str(s, 14);
        assert!(result2.len() <= 14);
        assert!(s.is_char_boundary(result2.len()));
    }

    #[test]
    fn test_truncate_emoji_utf8_safe() {
        // 🎉 is 4 bytes
        let s = "Hi 🎉🎊";
        // Truncate at 5 — in the middle of 🎉 (bytes 3-6) → should go back to 3
        let result = truncate_str(s, 5);
        assert!(result.len() <= 5);
        assert!(s.is_char_boundary(result.len()));
        assert_eq!(result, "Hi ");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate_str("", 100), "");
    }

    #[test]
    fn test_strip_markdown_fences_json() {
        assert_eq!(strip_markdown_fences("```json\n{\"key\": \"value\"}\n```"), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_strip_markdown_fences_plain() {
        assert_eq!(strip_markdown_fences("```\nsome code\n```"), "some code");
    }

    #[test]
    fn test_strip_markdown_fences_no_fences() {
        assert_eq!(strip_markdown_fences("plain text"), "plain text");
    }

    #[test]
    fn test_strip_markdown_fences_with_whitespace() {
        assert_eq!(strip_markdown_fences("  ```json\n{}\n```  "), "{}");
    }
}
