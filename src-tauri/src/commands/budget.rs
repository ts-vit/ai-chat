// Cost calculation, cost_ledger writes, and budget checks
use sqlx::Row;
use sqlx::SqlitePool;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

const STORE_NAME: &str = "settings.json";

type Pool = sqlx::SqlitePool;

/// Compute cost from model_catalog for given model. Returns 0.0 if model not found or costs are NULL.
/// catalog_id: e.g. "openrouter:anthropic/claude-sonnet-4" or "ollama:qwen2.5:3b"
pub async fn compute_cost_from_catalog(
    pool: &SqlitePool,
    catalog_id: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
) -> f64 {
    let row = sqlx::query(
        "SELECT cost_per_input_token, cost_per_output_token FROM model_catalog WHERE id = ? AND is_available = 1",
    )
    .bind(catalog_id)
    .fetch_optional(pool)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        _ => return 0.0,
    };

    let in_cost: Option<f64> = row.try_get("cost_per_input_token").ok();
    let out_cost: Option<f64> = row.try_get("cost_per_output_token").ok();

    match (in_cost, out_cost) {
        (Some(in_c), Some(out_c)) => {
            (prompt_tokens as f64 * in_c) + (completion_tokens as f64 * out_c)
        }
        _ => 0.0,
    }
}

/// Build catalog id from provider and raw model_id (as sent to API)
pub fn catalog_id(provider: &str, model_id: &str) -> String {
    format!("{}:{}", provider, model_id)
}

/// Infer provider from base URL (e.g. for cost_ledger and catalog lookup)
pub fn provider_from_base_url(base_url: &str) -> &'static str {
    let b = base_url.to_lowercase();
    if b.contains("openrouter") {
        "openrouter"
    } else if b.contains("11434") || b.contains("ollama") {
        "ollama"
    } else {
        "custom"
    }
}

/// Append a row to cost_ledger. Source: "chat" | "agent" | "plan_task" | "routing_decision" | "skill_detection"
pub async fn write_cost_ledger(
    pool: &SqlitePool,
    chat_id: &str,
    agent_run_id: Option<&str>,
    plan_id: Option<&str>,
    model_id: &str,
    provider: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
    source: &str,
) -> Result<(), String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query(
        "INSERT INTO cost_ledger (timestamp, chat_id, agent_run_id, plan_id, model_id, provider, prompt_tokens, completion_tokens, cost, source) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(timestamp)
    .bind(chat_id)
    .bind(agent_run_id)
    .bind(plan_id)
    .bind(model_id)
    .bind(provider)
    .bind(prompt_tokens as i64)
    .bind(completion_tokens as i64)
    .bind(cost)
    .bind(source)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Check budget before an LLM call. Returns Ok(()) if under limits, Err with message if over.
/// Emits budget-warning or budget-exceeded event. plan_id = None for regular chat/agent (no plan).
pub async fn check_budget(
    pool: &SqlitePool,
    app: &AppHandle,
    chat_id: &str,
    plan_id: Option<&str>,
) -> Result<(), String> {
    let store = app.store(STORE_NAME).map_err(|e: tauri_plugin_store::Error| e.to_string())?;

    let plan_enabled: bool = store.get("budgetPlanEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let plan_limit: f64 = store.get("budgetPlanLimit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let global_enabled: bool = store.get("budgetGlobalEnabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let global_limit: f64 = store.get("budgetGlobalLimit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let period: String = store.get("budgetGlobalPeriod").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "daily".to_string());

    if let Some(pid) = plan_id {
        if plan_enabled && plan_limit > 0.0 {
            let spend: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE plan_id = ?")
                .bind(pid)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .and_then(|row| row.try_get::<f64, _>("total").ok())
                .unwrap_or(0.0);
            if spend >= plan_limit {
                let _ = app.emit("budget-exceeded", serde_json::json!({
                    "level": "plan",
                    "currentSpend": spend,
                    "limit": plan_limit,
                    "planId": pid,
                }));
                return Err(format!("Budget exceeded: plan spend ${:.4} >= ${:.2} limit", spend, plan_limit));
            }
        }
    }

    if global_enabled && global_limit > 0.0 {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
        let (start_ts, _) = if period == "monthly" {
            let year = (now / 31536000) * 31536000;
            let month = ((now % 31536000) / 2592000) * 2592000;
            (year + month, ())
        } else {
            ((now / 86400) * 86400, ())
        };
        let spend: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE timestamp >= ?")
            .bind(start_ts)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<f64, _>("total").ok())
            .unwrap_or(0.0);
        if spend >= global_limit {
            let _ = app.emit("budget-exceeded", serde_json::json!({
                "level": "global",
                "currentSpend": spend,
                "limit": global_limit,
            }));
            return Err(format!("Budget exceeded: global spend ${:.4} >= ${:.2} limit", spend, global_limit));
        }
    }

    Ok(())
}

/// Get current spend for plan (if plan_id provided) and/or global period for UI.
pub async fn get_budget_status(
    pool: &SqlitePool,
    app: &AppHandle,
    plan_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let store = app.store(STORE_NAME).map_err(|e: tauri_plugin_store::Error| e.to_string())?;
    let plan_limit: f64 = store.get("budgetPlanLimit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let global_limit: f64 = store.get("budgetGlobalLimit").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let period: String = store.get("budgetGlobalPeriod").and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| "daily".to_string());

    let plan_spend: f64 = if let Some(ref pid) = plan_id {
        sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE plan_id = ?")
            .bind(pid)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            .and_then(|row| row.try_get::<f64, _>("total").ok())
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    let start_ts = if period == "monthly" {
        let year = (now / 31536000) * 31536000;
        let month = ((now % 31536000) / 2592000) * 2592000;
        year + month
    } else {
        (now / 86400) * 86400
    };
    let global_spend: f64 = sqlx::query("SELECT COALESCE(SUM(cost), 0) as total FROM cost_ledger WHERE timestamp >= ?")
        .bind(start_ts)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|row| row.try_get::<f64, _>("total").ok())
        .unwrap_or(0.0);

    Ok(serde_json::json!({
        "planSpend": plan_spend,
        "planLimit": plan_limit,
        "globalSpend": global_spend,
        "globalLimit": global_limit,
        "period": period,
    }))
}

#[tauri::command]
pub async fn get_budget_status_command(
    app: AppHandle,
    pool: State<'_, Pool>,
    plan_id: Option<String>,
) -> Result<serde_json::Value, String> {
    get_budget_status(pool.inner(), &app, plan_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_id_format() {
        assert_eq!(catalog_id("openrouter", "anthropic/claude-sonnet-4"), "openrouter:anthropic/claude-sonnet-4");
    }

    #[test]
    fn test_catalog_id_empty() {
        assert_eq!(catalog_id("", ""), ":");
    }

    #[test]
    fn test_catalog_id_ollama() {
        assert_eq!(catalog_id("ollama", "qwen2.5:3b"), "ollama:qwen2.5:3b");
    }

    #[test]
    fn test_provider_openrouter() {
        assert_eq!(provider_from_base_url("https://openrouter.ai/api/v1"), "openrouter");
    }

    #[test]
    fn test_provider_ollama_port() {
        assert_eq!(provider_from_base_url("http://localhost:11434"), "ollama");
    }

    #[test]
    fn test_provider_ollama_name() {
        assert_eq!(provider_from_base_url("http://ollama.local/api"), "ollama");
    }

    #[test]
    fn test_provider_custom_fallback() {
        assert_eq!(provider_from_base_url("https://api.example.com/v1"), "custom");
    }

    #[test]
    fn test_provider_case_insensitive() {
        assert_eq!(provider_from_base_url("https://OPENROUTER.ai/api"), "openrouter");
    }
}
