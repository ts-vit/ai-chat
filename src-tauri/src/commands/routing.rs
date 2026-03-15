// Routing rules and LLM-based model selection
use sqlx::Row;
use tauri::State;

use crate::commands::budget::{catalog_id, compute_cost_from_catalog, provider_from_base_url, write_cost_ledger};
use crate::services::http_client::build_http_client;

type Pool = sqlx::SqlitePool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingRule {
    pub id: i64,
    pub task_category: String,
    pub preferred_model: String,
    pub fallback_model: Option<String>,
    pub priority: i64,
    pub enabled: bool,
}

const ROUTING_PROMPT: &str = r#"You are a model router. Given the task description and list of available models with their strengths, select the best model.

Available models:
{models_json}

Task: {task_description}

Respond with ONLY the model ID (the id field from the list), nothing else."#;

#[tauri::command]
pub async fn get_routing_rules(pool: State<'_, Pool>) -> Result<Vec<RoutingRule>, String> {
    let rows = sqlx::query(
        "SELECT id, task_category, preferred_model, fallback_model, priority, enabled FROM routing_rules ORDER BY priority DESC, id",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|row| RoutingRule {
            id: row.get("id"),
            task_category: row.get("task_category"),
            preferred_model: row.get("preferred_model"),
            fallback_model: row.try_get("fallback_model").ok(),
            priority: row.get("priority"),
            enabled: row.get::<i32, _>("enabled") != 0,
        })
        .collect())
}

#[tauri::command]
pub async fn create_routing_rule(
    pool: State<'_, Pool>,
    task_category: String,
    preferred_model: String,
    fallback_model: Option<String>,
    priority: Option<i64>,
    enabled: Option<bool>,
) -> Result<RoutingRule, String> {
    let priority = priority.unwrap_or(0);
    let enabled = enabled.unwrap_or(true);
    sqlx::query(
        "INSERT INTO routing_rules (task_category, preferred_model, fallback_model, priority, enabled) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&task_category)
    .bind(&preferred_model)
    .bind(&fallback_model)
    .bind(priority)
    .bind(if enabled { 1 } else { 0 })
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    Ok(RoutingRule {
        id,
        task_category,
        preferred_model,
        fallback_model,
        priority,
        enabled,
    })
}

#[tauri::command]
pub async fn update_routing_rule(
    pool: State<'_, Pool>,
    id: i64,
    task_category: Option<String>,
    preferred_model: Option<String>,
    fallback_model: Option<String>,
    priority: Option<i64>,
    enabled: Option<bool>,
) -> Result<(), String> {
    let row = sqlx::query("SELECT task_category, preferred_model, fallback_model, priority, enabled FROM routing_rules WHERE id = ?")
        .bind(id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Rule not found".to_string())?;

    let mut cat: String = row.get("task_category");
    let mut pref: String = row.get("preferred_model");
    let mut fall: Option<String> = row.try_get("fallback_model").ok();
    let mut pri: i64 = row.get("priority");
    let mut en: i32 = row.get("enabled");

    if let Some(c) = task_category {
        cat = c;
    }
    if let Some(p) = preferred_model {
        pref = p;
    }
    if fallback_model.is_some() {
        fall = fallback_model;
    }
    if let Some(p) = priority {
        pri = p;
    }
    if let Some(e) = enabled {
        en = if e { 1 } else { 0 };
    }

    sqlx::query(
        "UPDATE routing_rules SET task_category = ?, preferred_model = ?, fallback_model = ?, priority = ?, enabled = ? WHERE id = ?",
    )
    .bind(&cat)
    .bind(&pref)
    .bind(&fall)
    .bind(pri)
    .bind(en)
    .bind(id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_routing_rule(pool: State<'_, Pool>, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM routing_rules WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn reset_routing_rules_to_defaults(pool: State<'_, Pool>) -> Result<(), String> {
    sqlx::query("DELETE FROM routing_rules")
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    for (cat, priority) in [
        ("coding", 10),
        ("writing", 9),
        ("analysis", 8),
        ("vision", 7),
        ("fast", 6),
        ("general", 0),
    ] {
        sqlx::query(
            "INSERT INTO routing_rules (task_category, preferred_model, fallback_model, priority, enabled) VALUES (?, '', NULL, ?, 1)",
        )
        .bind(cat)
        .bind(priority)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Rule-based: get preferred or fallback model for category. Returns catalog id (e.g. openrouter:model/id).
/// Empty preferred_model means "use chat default" -> None.
pub async fn get_model_for_category(
    pool: &Pool,
    category: &str,
) -> Option<String> {
    let row = sqlx::query(
        "SELECT preferred_model, fallback_model FROM routing_rules WHERE task_category = ? AND enabled = 1 ORDER BY priority DESC LIMIT 1",
    )
    .bind(category)
    .fetch_optional(pool)
    .await
    .ok()?;

    let row = row?;
    let preferred: String = row.get("preferred_model");
    if !preferred.is_empty() {
        let avail: i32 = sqlx::query_scalar("SELECT is_available FROM model_catalog WHERE id = ?")
            .bind(&preferred)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);
        if avail != 0 {
            return Some(preferred);
        }
    }
    let fallback: Option<String> = row.try_get("fallback_model").ok();
    if let Some(f) = fallback.filter(|s| !s.is_empty()) {
        let avail: i32 = sqlx::query_scalar("SELECT is_available FROM model_catalog WHERE id = ?")
            .bind(&f)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);
        if avail != 0 {
            return Some(f);
        }
    }
    None
}

/// LLM-based routing: call orchestrator model to pick best model for task. Returns catalog id or None.
/// Writes routing call cost to cost_ledger (source = "routing_decision").
pub async fn llm_route_task(
    pool: &Pool,
    app: &tauri::AppHandle,
    chat_id: &str,
    api_key: &str,
    base_url: &str,
    model: &str,
    task_description: &str,
) -> Result<Option<String>, String> {
    let rows = sqlx::query(
        "SELECT id, category, COALESCE(strengths, '') as strengths FROM model_catalog WHERE is_available = 1 ORDER BY display_name LIMIT 30",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let models: Vec<(String, String, String)> = rows
        .into_iter()
        .map(|row| {
            (
                row.get("id"),
                row.get("category"),
                row.get("strengths"),
            )
        })
        .collect();

    let models_json: String = models
        .iter()
        .map(|(id, cat, strengths)| format!("- id: {}, category: {}, strengths: {}", id, cat, strengths))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = ROUTING_PROMPT
        .replace("{models_json}", &models_json)
        .replace("{task_description}", task_description);

    let base = if base_url.is_empty() {
        "https://openrouter.ai/api/v1"
    } else {
        base_url.trim_end_matches('/')
    };
    let url = format!("{}/chat/completions", base);

    let client = build_http_client(app, None).await.map_err(|e| e.to_string())?;
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 10,
        "temperature": 0
    });

    let mut req = client.post(&url).json(&body);
    if !api_key.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", api_key));
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let resp_json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let content = resp_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    let usage = resp_json.get("usage");
    let pt = usage.and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let ct = usage.and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    let provider = provider_from_base_url(base);
    let cat_id = catalog_id(provider, model);
    let cost = compute_cost_from_catalog(pool, &cat_id, pt, ct).await;
    let _ = write_cost_ledger(
        pool,
        chat_id,
        None,
        None,
        model,
        provider,
        pt,
        ct,
        cost,
        "routing_decision",
    )
    .await;

    let chosen = content.lines().next().unwrap_or("").trim();
    if chosen.is_empty() {
        return Ok(None);
    }
    let valid = models.iter().any(|(id, _, _)| id == chosen);
    if valid {
        Ok(Some(chosen.to_string()))
    } else {
        Ok(None)
    }
}
