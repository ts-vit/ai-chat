// Model catalog: sync from OpenRouter/Ollama, get/update entries
use std::sync::Arc;
use sqlx::Row;
use tauri::{AppHandle, Manager, State};
use uni_settings::{JsonSettingsStore, SettingsStore};

use crate::commands::budget::catalog_id;
use crate::models::catalog::ModelCatalogEntry;
use crate::services::http_client::build_http_client;

type Pool = sqlx::SqlitePool;
type SettingsState = Arc<JsonSettingsStore>;

/// Heuristic category from OpenRouter model name/id
fn category_from_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("code") || lower.contains("codellama") || lower.contains("deepseek-coder") {
        "coding"
    } else if lower.contains("vision") || lower.contains("llava") || lower.contains("gpt-4o") || lower.contains("claude-3-5") {
        "vision"
    } else if lower.contains("fast") || lower.contains("flash") || lower.contains("mini") || lower.contains(":3b") || lower.contains(":7b") {
        "fast"
    } else if lower.contains("writing") || lower.contains("grammar") {
        "writing"
    } else if lower.contains("analysis") || lower.contains("reason") || lower.contains("o1") || lower.contains("o3") {
        "analysis"
    } else {
        "general"
    }
}

/// Category from Ollama model name/family
fn ollama_category(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.contains("code") || lower.contains("codellama") || lower.contains("deepseek-coder") || lower.contains("qwen2.5-coder") {
        "coding"
    } else if lower.contains("llava") || lower.contains("vision") {
        "vision"
    } else if lower.contains(":3b") || lower.contains(":7b") || lower.contains("tiny") || lower.contains("mini") {
        "fast"
    } else if lower.contains("qwen") || lower.contains("llama") || lower.contains("mistral") {
        "general"
    } else {
        "general"
    }
}

#[tauri::command]
pub async fn sync_model_catalog(app: AppHandle, pool: State<'_, Pool>) -> Result<i64, String> {
    let client = build_http_client(&app, None).await?;
    let now = uni_common::now_unix_secs();
    let mut synced_count = 0;

    // 1. OpenRouter
    let response = client
        .get("https://openrouter.ai/api/v1/models")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let empty: Vec<serde_json::Value> = vec![];
        let data = json.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut openrouter_ids = Vec::new();

        for item in data {
            let id = match item.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();
            let context_length = item.get("context_length").and_then(|v| v.as_i64()).unwrap_or(0);
            let pricing = item.get("pricing");
            let (cost_in, cost_out) = if let Some(p) = pricing {
                let obj = p.as_object().or_else(|| p.as_array().and_then(|a| a.first()).and_then(|t| t.as_object()));
                match obj {
                    Some(o) => {
                        let prompt = o.get("prompt").and_then(|v| v.as_str()).unwrap_or("0");
                        let completion = o.get("completion").and_then(|v| v.as_str()).unwrap_or("0");
                        (prompt.parse::<f64>().unwrap_or(0.0), completion.parse::<f64>().unwrap_or(0.0))
                    }
                    None => (0.0, 0.0),
                }
            } else {
                (0.0, 0.0)
            };

            let catalog_id_str = catalog_id("openrouter", &id);
            openrouter_ids.push(catalog_id_str.clone());

            sqlx::query(
                "INSERT INTO model_catalog (id, provider, model_id, display_name, cost_per_input_token, cost_per_output_token, context_window, category, strengths, is_available, updated_at)
                 VALUES (?, 'openrouter', ?, ?, ?, ?, ?, ?, NULL, 1, ?)
                 ON CONFLICT(id) DO UPDATE SET
                   display_name = excluded.display_name,
                   cost_per_input_token = excluded.cost_per_input_token,
                   cost_per_output_token = excluded.cost_per_output_token,
                   context_window = excluded.context_window,
                   category = excluded.category,
                   is_available = 1,
                   updated_at = excluded.updated_at",
            )
            .bind(&catalog_id_str)
            .bind(&id)
            .bind(&name)
            .bind(cost_in)
            .bind(cost_out)
            .bind(context_length)
            .bind(category_from_name(&name))
            .bind(now)
            .execute(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
            synced_count += 1;
        }

        // Mark OpenRouter models not in response as unavailable
        for row in sqlx::query("SELECT id FROM model_catalog WHERE provider = 'openrouter'")
            .fetch_all(pool.inner())
            .await
            .unwrap_or_default()
        {
            let id: String = row.get("id");
            if !openrouter_ids.contains(&id) {
                let _ = sqlx::query("UPDATE model_catalog SET is_available = 0, updated_at = ? WHERE id = ?")
                    .bind(now)
                    .bind(&id)
                    .execute(pool.inner())
                    .await;
            }
        }
    }

    // 2. Ollama (from store ollama_url)
    let settings = app.state::<SettingsState>();
    let ollama_url: String = settings
        .get("llm.ollama.url")
        .await
        .unwrap_or_default()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:11434/v1".to_string());

    let base = ollama_url.trim_end_matches('/').replace("/v1", "");
    let tags_url = if base.is_empty() {
        "http://localhost:11434/api/tags".to_string()
    } else {
        format!("{}/api/tags", base)
    };

    if let Ok(resp) = client.get(&tags_url).send().await {
        if resp.status().is_success() {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let empty: Vec<serde_json::Value> = vec![];
                let models = json.get("models").and_then(|m| m.as_array()).unwrap_or(&empty);
                for item in models {
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let catalog_id_str = catalog_id("ollama", &name);
                    let display = item.get("name").and_then(|v| v.as_str()).unwrap_or(&name).to_string();
                    sqlx::query(
                        "INSERT INTO model_catalog (id, provider, model_id, display_name, cost_per_input_token, cost_per_output_token, context_window, category, strengths, is_available, updated_at)
                         VALUES (?, 'ollama', ?, ?, 0.0, 0.0, NULL, ?, NULL, 1, ?)
                         ON CONFLICT(id) DO UPDATE SET
                           display_name = excluded.display_name,
                           is_available = 1,
                           updated_at = excluded.updated_at",
                    )
                    .bind(&catalog_id_str)
                    .bind(&name)
                    .bind(&display)
                    .bind(ollama_category(&name))
                    .bind(now)
                    .execute(pool.inner())
                    .await
                    .map_err(|e| e.to_string())?;
                    synced_count += 1;
                }
            }
        }
    }

    // Update last sync timestamp in store
    let _ = settings.set("model.catalog.last_sync", &now.to_string()).await;

    Ok(synced_count as i64)
}

#[tauri::command]
pub async fn get_model_catalog(
    pool: State<'_, Pool>,
    provider: Option<String>,
    category: Option<String>,
    is_available: Option<bool>,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let rows = sqlx::query(
        "SELECT id, provider, model_id, display_name, cost_per_input_token, cost_per_output_token, context_window, category, strengths, is_available, updated_at FROM model_catalog ORDER BY provider, display_name",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        let prov: String = row.get("provider");
        let avail: i32 = row.get("is_available");
        if let Some(ref p) = provider {
            if &prov != p {
                continue;
            }
        }
        if let Some(av) = is_available {
            if (avail != 0) != av {
                continue;
            }
        }
        if let Some(ref c) = category {
            let cat: String = row.get("category");
            if &cat != c {
                continue;
            }
        }
        out.push(ModelCatalogEntry {
            id: row.get("id"),
            provider: row.get("provider"),
            model_id: row.get("model_id"),
            display_name: row.get("display_name"),
            cost_per_input_token: row.try_get("cost_per_input_token").ok(),
            cost_per_output_token: row.try_get("cost_per_output_token").ok(),
            context_window: row.try_get("context_window").ok(),
            category: row.get("category"),
            strengths: row.try_get("strengths").ok(),
            is_available: avail != 0,
            updated_at: row.get("updated_at"),
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn update_model_catalog_entry(
    pool: State<'_, Pool>,
    id: String,
    category: Option<String>,
    strengths: Option<String>,
    cost_per_input_token: Option<f64>,
    cost_per_output_token: Option<f64>,
    is_available: Option<bool>,
) -> Result<(), String> {
    let now = uni_common::now_unix_secs();
    let row = sqlx::query("SELECT category, strengths, cost_per_input_token, cost_per_output_token, is_available FROM model_catalog WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    let (mut cat, mut str_, mut cin, mut cout, mut avail) = match row {
        Some(r) => (
            r.get::<String, _>("category"),
            r.try_get::<String, _>("strengths").ok(),
            r.try_get::<f64, _>("cost_per_input_token").ok(),
            r.try_get::<f64, _>("cost_per_output_token").ok(),
            r.get::<i32, _>("is_available"),
        ),
        None => return Err("Model not found in catalog".to_string()),
    };

    if let Some(c) = category {
        cat = c;
    }
    if let Some(s) = strengths {
        str_ = Some(s);
    }
    if cost_per_input_token.is_some() {
        cin = cost_per_input_token;
    }
    if cost_per_output_token.is_some() {
        cout = cost_per_output_token;
    }
    if let Some(b) = is_available {
        avail = if b { 1 } else { 0 };
    }

    sqlx::query(
        "UPDATE model_catalog SET category = ?, strengths = ?, cost_per_input_token = ?, cost_per_output_token = ?, is_available = ?, updated_at = ? WHERE id = ?",
    )
    .bind(cat)
    .bind(&str_)
    .bind(cin)
    .bind(cout)
    .bind(avail)
    .bind(now)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
