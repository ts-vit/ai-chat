// Команды для работы с SQLite (чаты и сообщения)
use std::collections::HashMap;
use std::sync::Arc;
use sqlx::Row;
use tauri::{AppHandle, State};
use uuid::Uuid;

use crate::commands::attachments::delete_attachments_for_message;
use crate::models::chat::{DbChat, DbImageStyle, DbMessage, DbMessageWithSiblings, SiblingPreview};
use crate::services::fts;
use crate::services::vector_store::VectorStore;

type Pool = sqlx::SqlitePool;

#[tauri::command]
pub async fn create_chat(
    pool: State<'_, Pool>,
    title: String,
    system_prompt: Option<String>,
    provider_id: Option<String>,
    model: Option<String>,
    is_image_model: Option<bool>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    image_size: Option<String>,
    image_quality: Option<String>,
    image_style: Option<String>,
    image_n: Option<u32>,
    negative_prompt: Option<String>,
    mode: Option<String>,
    project_id: Option<String>,
) -> Result<DbChat, String> {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    log::debug!(
        "create_chat: provider={:?}, model={:?}, temperature={:?}",
        provider_id,
        model,
        temperature
    );

    let system_prompt_str = system_prompt.unwrap_or_default();
    let provider = provider_id
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "openrouter".to_string());
    let model_str = model.unwrap_or_default();
    let is_image = is_image_model.unwrap_or(false) as i64;

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, system_prompt, provider_id, model, is_image_model, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, image_size, image_quality, image_style, image_n, negative_prompt, mode, project_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&title)
    .bind(now)
    .bind(now)
    .bind(&system_prompt_str)
    .bind(&provider)
    .bind(&model_str)
    .bind(is_image)
    .bind(temperature)
    .bind(max_tokens.map(|v| v as i64))
    .bind(top_p)
    .bind(top_k.map(|v| v as i64))
    .bind(frequency_penalty)
    .bind(presence_penalty)
    .bind(&image_size)
    .bind(&image_quality)
    .bind(&image_style)
    .bind(image_n.map(|v| v as i64))
    .bind(&negative_prompt)
    .bind(mode.clone().unwrap_or_else(|| "chat".to_string()))
    .bind(&project_id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[create_chat] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbChat {
        id: id.clone(),
        title,
        created_at: now,
        updated_at: now,
        system_prompt: Some(system_prompt_str.clone()).filter(|s| !s.is_empty()),
        provider_id: provider,
        model: model_str,
        folder_id: None,
        project_id: project_id.clone(),
        is_image_model: is_image_model.unwrap_or(false),
        temperature,
        max_tokens,
        top_p,
        top_k,
        frequency_penalty,
        presence_penalty,
        image_size,
        image_quality,
        image_style,
        image_n,
        negative_prompt,
        active_child_map: Some("{}".to_string()),
        mode: mode.unwrap_or_else(|| "chat".to_string()),
    })
}

#[tauri::command]
pub async fn delete_chat(app: AppHandle, pool: State<'_, Pool>, id: String) -> Result<(), String> {
    let rows = sqlx::query("SELECT id, content FROM messages WHERE chat_id = ? AND has_attachments = 1")
        .bind(&id)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_chat] SQL error (fetch): {}", e);
            e.to_string()
        })?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    // Delete workspace artifacts for this chat
    let _ = sqlx::query("DELETE FROM workspace_artifacts WHERE chat_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    sqlx::query("DELETE FROM chats WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_chat] SQL error (delete): {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn get_all_chats(pool: State<'_, Pool>) -> Result<Vec<DbChat>, String> {
    let rows = sqlx::query(
        "SELECT id, title, created_at, updated_at, system_prompt, provider_id, model, folder_id, project_id, is_image_model, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, image_size, image_quality, image_style, image_n, negative_prompt, active_child_map, mode FROM chats ORDER BY updated_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_all_chats] SQL error: {}", e);
        e.to_string()
    })?;

    let chats: Vec<DbChat> = rows
        .into_iter()
        .map(|row| DbChat {
            id: row.get("id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            system_prompt: row.try_get::<String, _>("system_prompt").ok(),
            provider_id: row.try_get::<String, _>("provider_id").unwrap_or_else(|_| "openrouter".to_string()),
            model: row.try_get::<String, _>("model").unwrap_or_default(),
            folder_id: row.try_get::<String, _>("folder_id").ok(),
            project_id: row.try_get::<String, _>("project_id").ok(),
            is_image_model: row.try_get::<i64, _>("is_image_model").unwrap_or(0) != 0,
            temperature: row.try_get::<Option<f32>, _>("temperature").ok().flatten(),
            max_tokens: row.try_get::<Option<i64>, _>("max_tokens").ok().flatten().map(|v| v as u32),
            top_p: row.try_get::<Option<f32>, _>("top_p").ok().flatten(),
            top_k: row.try_get::<Option<i64>, _>("top_k").ok().flatten().map(|v| v as u32),
            frequency_penalty: row.try_get::<Option<f32>, _>("frequency_penalty").ok().flatten(),
            presence_penalty: row.try_get::<Option<f32>, _>("presence_penalty").ok().flatten(),
            image_size: row.try_get::<Option<String>, _>("image_size").ok().flatten(),
            image_quality: row.try_get::<Option<String>, _>("image_quality").ok().flatten(),
            image_style: row.try_get::<Option<String>, _>("image_style").ok().flatten(),
            image_n: row.try_get::<Option<i64>, _>("image_n").ok().flatten().map(|v| v as u32),
            negative_prompt: row.try_get::<Option<String>, _>("negative_prompt").ok().flatten(),
            active_child_map: row.try_get::<Option<String>, _>("active_child_map").ok().flatten(),
            mode: row.try_get::<String, _>("mode").unwrap_or_else(|_| "chat".to_string()),
        })
        .collect();
    log::debug!("get_all_chats: loaded {} chats", chats.len());
    Ok(chats)
}

#[tauri::command]
pub async fn save_message(
    pool: State<'_, Pool>,
    chat_id: String,
    role: String,
    content: String,
    parent_id: Option<String>,
    timestamp: i64,
    model: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
) -> Result<DbMessage, String> {
    let id = Uuid::new_v4().to_string();
    let pt = prompt_tokens as i64;
    let ct = completion_tokens as i64;

    sqlx::query(
        "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
    )
    .bind(&id)
    .bind(&chat_id)
    .bind(&role)
    .bind(&content)
    .bind(&parent_id)
    .bind(timestamp)
    .bind(&model)
    .bind(pt)
    .bind(ct)
    .bind(cost)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[save_message] SQL error: {}", e);
        e.to_string()
    })?;

    // Update active_child_map so the new message becomes the active child of its parent
    let map_key = parent_id.clone().unwrap_or_default(); // None → ""
    if let Ok(row) = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_one(pool.inner())
        .await
    {
        let map_str: String = row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string());
        let mut map: HashMap<String, String> = serde_json::from_str(&map_str).unwrap_or_default();
        map.insert(map_key, id.clone());
        let new_map = serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string());
        let _ = sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
            .bind(&new_map)
            .bind(&chat_id)
            .execute(pool.inner())
            .await;
    }

    Ok(DbMessage {
        id: id.clone(),
        chat_id,
        role,
        content,
        parent_id,
        timestamp,
        model: if model.is_empty() { None } else { Some(model) },
        prompt_tokens: Some(pt),
        completion_tokens: Some(ct),
        cost: Some(cost),
        has_attachments: Some(0),
        web_sources: None,
        agent_step: None,
        agent_run_id: None,
    })
}

#[tauri::command]
pub async fn update_message_usage(
    pool: State<'_, Pool>,
    id: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
    chat_id: Option<String>,
    model_id: Option<String>,
    provider: Option<String>,
) -> Result<(), String> {
    let cost_to_store = if let (Some(cid), Some(mid), Some(prov)) = (chat_id, model_id, provider) {
        let catalog_id = crate::commands::budget::catalog_id(&prov, &mid);
        let computed = crate::commands::budget::compute_cost_from_catalog(pool.inner(), &catalog_id, prompt_tokens, completion_tokens).await;
        let _ = crate::commands::budget::write_cost_ledger(
            pool.inner(),
            &cid,
            None,
            None,
            &mid,
            &prov,
            prompt_tokens,
            completion_tokens,
            computed,
            "chat",
        ).await;
        computed
    } else {
        cost
    };

    sqlx::query(
        "UPDATE messages SET prompt_tokens = ?, completion_tokens = ?, cost = ? WHERE id = ?",
    )
    .bind(prompt_tokens as i64)
    .bind(completion_tokens as i64)
    .bind(cost_to_store)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[update_message_usage] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

#[tauri::command]
pub async fn update_message_content(
    pool: State<'_, Pool>,
    id: String,
    content: String,
) -> Result<(), String> {
    let has_attachments = content
        .trim_start()
        .starts_with('[')
        .then(|| {
            serde_json::from_str::<Vec<serde_json::Value>>(&content)
                .ok()
                .map(|arr| arr.iter().any(|b| b.get("path").is_some()))
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if has_attachments {
        sqlx::query("UPDATE messages SET content = ?, has_attachments = 1 WHERE id = ?")
            .bind(&content)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                let msg = e.to_string();
                log::error!("[update_message_content] SQL error (has_attachments): {}", msg);
                msg
            })?;
    } else {
        sqlx::query("UPDATE messages SET content = ? WHERE id = ?")
            .bind(&content)
            .bind(&id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                let msg = e.to_string();
                log::error!("[update_message_content] SQL error: {}", msg);
                msg
            })?;
    }
    Ok(())
}

/// Обновляет content сообщения и выставляет has_attachments = 1 (для сообщений с вложениями).
pub async fn update_message_content_with_attachments(
    pool: &Pool,
    id: &str,
    content: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE messages SET content = ?, has_attachments = 1 WHERE id = ?")
        .bind(content)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            log::error!("[update_message_content_with_attachments] SQL error: {}", msg);
            msg
        })?;
    Ok(())
}

#[tauri::command]
pub async fn get_messages(pool: State<'_, Pool>, chat_id: String) -> Result<Vec<DbMessage>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, web_sources, agent_step, agent_run_id FROM messages WHERE chat_id = ? ORDER BY timestamp",
    )
    .bind(&chat_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_messages] SQL error: {}", e);
        e.to_string()
    })?;

    let messages = rows
        .into_iter()
        .map(|row| DbMessage {
            id: row.get("id"),
            chat_id: row.get("chat_id"),
            role: row.get("role"),
            content: row.get("content"),
            parent_id: row.get("parent_id"),
            timestamp: row.get("timestamp"),
            model: row.try_get("model").ok(),
            prompt_tokens: row.try_get("prompt_tokens").ok(),
            completion_tokens: row.try_get("completion_tokens").ok(),
            cost: row.try_get("cost").ok(),
            has_attachments: row.try_get("has_attachments").ok(),
            web_sources: row.try_get::<Option<String>, _>("web_sources").ok().flatten(),
            agent_step: row.try_get("agent_step").ok().flatten(),
            agent_run_id: row.try_get::<Option<String>, _>("agent_run_id").ok().flatten(),
        })
        .collect();
    Ok(messages)
}

#[tauri::command]
pub async fn delete_messages_after(
    app: AppHandle,
    pool: State<'_, Pool>,
    chat_id: String,
    timestamp: i64,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<(), String> {
    let ids_to_delete: Vec<String> = sqlx::query_scalar("SELECT id FROM messages WHERE chat_id = ? AND timestamp > ?")
        .bind(&chat_id)
        .bind(timestamp)
        .fetch_all(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_messages_after] SQL error (fetch ids): {}", e);
            e.to_string()
        })?;
    for id in &ids_to_delete {
        if let Some(s) = store.as_ref().as_ref() {
            let _ = s.delete_by_message_id(id).await;
        }
        let _ = fts::fts_delete_message(pool.inner(), id).await;
        let _ = sqlx::query("UPDATE messages SET fts_indexed = 0 WHERE id = ?")
            .bind(id)
            .execute(pool.inner())
            .await;
    }
    let rows = sqlx::query(
        "SELECT id, content FROM messages WHERE chat_id = ? AND timestamp > ? AND has_attachments = 1",
    )
    .bind(&chat_id)
    .bind(timestamp)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[delete_messages_after] SQL error (fetch attachments): {}", e);
        e.to_string()
    })?;
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        delete_attachments_for_message(&app, &content);
    }
    sqlx::query("DELETE FROM messages WHERE chat_id = ? AND timestamp > ?")
        .bind(&chat_id)
        .bind(timestamp)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_messages_after] SQL error (delete): {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_title(
    pool: State<'_, Pool>,
    chat_id: String,
    title: String,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    sqlx::query("UPDATE chats SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&title)
        .bind(now)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_chat_title] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_model(
    pool: State<'_, Pool>,
    chat_id: String,
    model: String,
    is_image_model: Option<bool>,
) -> Result<(), String> {
    let is_image = is_image_model.unwrap_or(false) as i64;
    sqlx::query("UPDATE chats SET model = ?, is_image_model = ?, updated_at = strftime('%s', 'now') WHERE id = ?")
        .bind(&model)
        .bind(is_image)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_chat_model] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_params(
    pool: State<'_, Pool>,
    chat_id: String,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE chats SET temperature = ?, max_tokens = ?, top_p = ?, top_k = ?, frequency_penalty = ?, presence_penalty = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
    )
    .bind(temperature)
    .bind(max_tokens.map(|v| v as i64))
    .bind(top_p)
    .bind(top_k.map(|v| v as i64))
    .bind(frequency_penalty)
    .bind(presence_penalty)
    .bind(&chat_id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[update_chat_params] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_image_config(
    pool: State<'_, Pool>,
    chat_id: String,
    image_size: Option<String>,
    image_quality: Option<String>,
    image_style: Option<String>,
    image_n: Option<u32>,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE chats SET image_size = ?, image_quality = ?, image_style = ?, image_n = ?, updated_at = strftime('%s', 'now') WHERE id = ?",
    )
    .bind(&image_size)
    .bind(&image_quality)
    .bind(&image_style)
    .bind(image_n.map(|v| v as i64))
    .bind(&chat_id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[update_chat_image_config] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

#[tauri::command]
pub async fn update_chat_negative_prompt(
    pool: State<'_, Pool>,
    chat_id: String,
    negative_prompt: Option<String>,
) -> Result<(), String> {
    sqlx::query("UPDATE chats SET negative_prompt = ?, updated_at = strftime('%s', 'now') WHERE id = ?")
        .bind(&negative_prompt)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_chat_negative_prompt] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn get_image_styles(pool: State<'_, Pool>) -> Result<Vec<DbImageStyle>, String> {
    let rows = sqlx::query(
        "SELECT id, name, prompt_suffix, is_builtin, sort_order, created_at FROM image_styles ORDER BY sort_order",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_image_styles] SQL error: {}", e);
        e.to_string()
    })?;

    let styles: Vec<DbImageStyle> = rows
        .into_iter()
        .map(|row| DbImageStyle {
            id: row.get("id"),
            name: row.get("name"),
            prompt_suffix: row.get("prompt_suffix"),
            is_builtin: row.try_get::<i64, _>("is_builtin").unwrap_or(0) != 0,
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
        })
        .collect();
    Ok(styles)
}

#[tauri::command]
pub async fn create_image_style(
    pool: State<'_, Pool>,
    name: String,
    prompt_suffix: String,
) -> Result<DbImageStyle, String> {
    let id = Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let max_order: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(sort_order), 0) FROM image_styles")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO image_styles (id, name, prompt_suffix, is_builtin, sort_order, created_at) VALUES (?, ?, ?, 0, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&prompt_suffix)
    .bind(max_order + 1)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[create_image_style] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbImageStyle {
        id,
        name,
        prompt_suffix,
        is_builtin: false,
        sort_order: max_order + 1,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_image_style(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    prompt_suffix: String,
) -> Result<(), String> {
    sqlx::query("UPDATE image_styles SET name = ?, prompt_suffix = ? WHERE id = ? AND is_builtin = 0")
        .bind(&name)
        .bind(&prompt_suffix)
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_image_style] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_image_style(
    pool: State<'_, Pool>,
    id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM image_styles WHERE id = ? AND is_builtin = 0")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_image_style] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

// --- Branching (message forking) ---

fn make_content_preview(content: &str) -> String {
    let cleaned: String = content
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.len() <= 80 {
        trimmed.to_string()
    } else {
        let mut end = 80;
        while !trimmed.is_char_boundary(end) && end < trimmed.len() {
            end += 1;
        }
        format!("{}…", &trimmed[..end])
    }
}

fn make_sibling_previews(siblings: &[&DbMessage]) -> Vec<SiblingPreview> {
    if siblings.len() <= 1 {
        return vec![];
    }
    siblings
        .iter()
        .map(|m| SiblingPreview {
            id: m.id.clone(),
            content_preview: make_content_preview(&m.content),
            created_at: m.timestamp,
        })
        .collect()
}

fn build_active_path(
    all_messages: &[DbMessage],
    active_child_map: &HashMap<String, String>,
) -> Vec<DbMessageWithSiblings> {
    // Build children map: parent_id → Vec<DbMessage> (sorted by timestamp via input order)
    let mut children_map: HashMap<Option<String>, Vec<&DbMessage>> = HashMap::new();
    for msg in all_messages {
        children_map.entry(msg.parent_id.clone()).or_default().push(msg);
    }

    let mut result: Vec<DbMessageWithSiblings> = Vec::new();

    // Find root: children of None (or empty string as fallback for legacy data)
    let roots = match children_map.get(&None) {
        Some(r) => r.clone(),
        None => match children_map.get(&Some(String::new())) {
            Some(r) => r.clone(),
            None => return result,
        },
    };

    // Pick active root
    let active_root = if roots.len() == 1 {
        roots[0]
    } else {
        // Multiple roots — check map with "" key
        if let Some(active_id) = active_child_map.get("") {
            roots.iter().find(|m| &m.id == active_id).copied().unwrap_or(roots[0])
        } else {
            roots[0]
        }
    };

    // Record root sibling info
    let root_index = roots.iter().position(|m| m.id == active_root.id).unwrap_or(0);
    result.push(DbMessageWithSiblings {
        message: active_root.clone(),
        sibling_count: roots.len() as u32,
        sibling_index: root_index as u32,
        sibling_ids: roots.iter().map(|m| m.id.clone()).collect(),
        siblings: make_sibling_previews(&roots),
    });

    // Walk the tree
    let mut current_id = active_root.id.clone();
    loop {
        let children = match children_map.get(&Some(current_id.clone())) {
            Some(c) if !c.is_empty() => c.clone(),
            _ => break,
        };

        // Pick active child
        let active_child = if children.len() == 1 {
            children[0]
        } else if let Some(active_id) = active_child_map.get(&current_id) {
            children.iter().find(|m| &m.id == active_id).copied()
                .unwrap_or(children.last().unwrap())
        } else {
            children.last().unwrap()
        };

        let child_index = children.iter().position(|m| m.id == active_child.id).unwrap_or(0);
        result.push(DbMessageWithSiblings {
            message: active_child.clone(),
            sibling_count: children.len() as u32,
            sibling_index: child_index as u32,
            sibling_ids: children.iter().map(|m| m.id.clone()).collect(),
            siblings: make_sibling_previews(&children),
        });

        current_id = active_child.id.clone();
    }

    result
}

#[tauri::command]
pub async fn get_messages_branched(
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<Vec<DbMessageWithSiblings>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, web_sources, agent_step, agent_run_id FROM messages WHERE chat_id = ? ORDER BY timestamp",
    )
    .bind(&chat_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_messages_branched] SQL error: {}", e);
        e.to_string()
    })?;

    let all_messages: Vec<DbMessage> = rows
        .iter()
        .map(|row| DbMessage {
            id: row.get("id"),
            chat_id: row.get("chat_id"),
            role: row.get("role"),
            content: row.get("content"),
            parent_id: row.try_get("parent_id").ok(),
            timestamp: row.get("timestamp"),
            model: row.try_get("model").ok(),
            prompt_tokens: row.try_get("prompt_tokens").ok(),
            completion_tokens: row.try_get("completion_tokens").ok(),
            cost: row.try_get("cost").ok(),
            has_attachments: row.try_get("has_attachments").ok(),
            web_sources: row.try_get("web_sources").ok(),
            agent_step: row.try_get("agent_step").ok().flatten(),
            agent_run_id: row.try_get::<Option<String>, _>("agent_run_id").ok().flatten(),
        })
        .collect();

    // Load active_child_map
    let map_str: String = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_one(pool.inner())
        .await
        .map(|row| row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|_| "{}".to_string());
    let active_child_map: HashMap<String, String> =
        serde_json::from_str(&map_str).unwrap_or_default();

    let result = build_active_path(&all_messages, &active_child_map);

    if result.is_empty() && !all_messages.is_empty() {
        log::warn!(
            "[get_messages_branched] fallback to flat list for chat {} ({} messages)",
            chat_id,
            all_messages.len()
        );
        Ok(all_messages
            .iter()
            .map(|m| DbMessageWithSiblings {
                message: m.clone(),
                sibling_count: 1,
                sibling_index: 0,
                sibling_ids: vec![m.id.clone()],
                siblings: vec![],
            })
            .collect())
    } else {
        Ok(result)
    }
}

#[tauri::command]
pub async fn switch_branch(
    pool: State<'_, Pool>,
    chat_id: String,
    message_id: String,
) -> Result<Vec<DbMessageWithSiblings>, String> {
    // Get parent_id of the target message
    let row = sqlx::query("SELECT parent_id FROM messages WHERE id = ?")
        .bind(&message_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[switch_branch] SQL error: {}", e);
            e.to_string()
        })?;
    let parent_id: Option<String> = row.try_get("parent_id").ok();
    let map_key = parent_id.unwrap_or_default(); // None → ""

    // Load and update active_child_map
    let map_row = sqlx::query("SELECT active_child_map FROM chats WHERE id = ?")
        .bind(&chat_id)
        .fetch_one(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
    let map_str: String = map_row.try_get("active_child_map").unwrap_or_else(|_| "{}".to_string());
    let mut active_child_map: HashMap<String, String> =
        serde_json::from_str(&map_str).unwrap_or_default();
    active_child_map.insert(map_key, message_id);

    let new_map = serde_json::to_string(&active_child_map).unwrap_or_else(|_| "{}".to_string());
    sqlx::query("UPDATE chats SET active_child_map = ? WHERE id = ?")
        .bind(&new_map)
        .bind(&chat_id)
        .execute(pool.inner())
        .await
        .map_err(|e| e.to_string())?;

    // Return the new active path
    get_messages_branched(pool, chat_id).await
}
