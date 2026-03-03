// CRUD для шаблонов чатов (chat_templates)
use sqlx::Row;
use tauri::State;
use uuid::Uuid;

use crate::models::chat::DbChatTemplate;

type Pool = sqlx::SqlitePool;

fn unix_now() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())
        .map(|d| d.as_secs() as i64)
}

#[tauri::command]
pub async fn get_all_templates(pool: State<'_, Pool>) -> Result<Vec<DbChatTemplate>, String> {
    let rows = sqlx::query(
        "SELECT id, name, icon, provider_id, model, system_prompt, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, sort_order, created_at FROM chat_templates ORDER BY sort_order, created_at",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_all_templates] SQL error: {}", e);
        e.to_string()
    })?;

    let list: Vec<DbChatTemplate> = rows
        .into_iter()
        .map(|row| DbChatTemplate {
            id: row.get("id"),
            name: row.get("name"),
            icon: row.get("icon"),
            provider_id: row.get("provider_id"),
            model: row.get("model"),
            system_prompt: row.get("system_prompt"),
            temperature: row.try_get::<Option<f32>, _>("temperature").ok().flatten(),
            max_tokens: row.try_get::<Option<i64>, _>("max_tokens").ok().flatten().map(|v| v as u32),
            top_p: row.try_get::<Option<f32>, _>("top_p").ok().flatten(),
            top_k: row.try_get::<Option<i64>, _>("top_k").ok().flatten().map(|v| v as u32),
            frequency_penalty: row.try_get::<Option<f32>, _>("frequency_penalty").ok().flatten(),
            presence_penalty: row.try_get::<Option<f32>, _>("presence_penalty").ok().flatten(),
            sort_order: row.get("sort_order"),
            created_at: row.get("created_at"),
        })
        .collect();
    log::debug!("get_all_templates: loaded {} templates", list.len());
    Ok(list)
}

#[tauri::command]
pub async fn create_template(
    pool: State<'_, Pool>,
    name: String,
    icon: Option<String>,
    provider_id: String,
    model: String,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
) -> Result<DbChatTemplate, String> {
    let id = Uuid::new_v4().to_string();
    let now = unix_now()?;
    let icon = icon.unwrap_or_else(|| "💬".to_string());
    let system_prompt = system_prompt.unwrap_or_default();
    let sort_order: i64 = 0;

    log::debug!("create_template: name={}, temperature={:?}", name, temperature);

    sqlx::query(
        "INSERT INTO chat_templates (id, name, icon, provider_id, model, system_prompt, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&icon)
    .bind(&provider_id)
    .bind(&model)
    .bind(&system_prompt)
    .bind(temperature)
    .bind(max_tokens.map(|v| v as i64))
    .bind(top_p)
    .bind(top_k.map(|v| v as i64))
    .bind(frequency_penalty)
    .bind(presence_penalty)
    .bind(sort_order)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[create_template] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbChatTemplate {
        id: id.clone(),
        name,
        icon,
        provider_id,
        model,
        system_prompt,
        temperature,
        max_tokens,
        top_p,
        top_k,
        frequency_penalty,
        presence_penalty,
        sort_order,
        created_at: now,
    })
}

#[tauri::command]
pub async fn update_template(
    pool: State<'_, Pool>,
    id: String,
    name: String,
    icon: Option<String>,
    provider_id: String,
    model: String,
    system_prompt: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
) -> Result<(), String> {
    let icon = icon.unwrap_or_else(|| "💬".to_string());
    let system_prompt = system_prompt.unwrap_or_default();

    sqlx::query(
        "UPDATE chat_templates SET name = ?, icon = ?, provider_id = ?, model = ?, system_prompt = ?, temperature = ?, max_tokens = ?, top_p = ?, top_k = ?, frequency_penalty = ?, presence_penalty = ? WHERE id = ?",
    )
    .bind(&name)
    .bind(&icon)
    .bind(&provider_id)
    .bind(&model)
    .bind(&system_prompt)
    .bind(temperature)
    .bind(max_tokens.map(|v| v as i64))
    .bind(top_p)
    .bind(top_k.map(|v| v as i64))
    .bind(frequency_penalty)
    .bind(presence_penalty)
    .bind(&id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[update_template] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}

#[tauri::command]
pub async fn delete_template(pool: State<'_, Pool>, id: String) -> Result<(), String> {
    sqlx::query("DELETE FROM chat_templates WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_template] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn reorder_templates(
    pool: State<'_, Pool>,
    template_ids: Vec<String>,
) -> Result<(), String> {
    for (index, template_id) in template_ids.into_iter().enumerate() {
        let sort_order = index as i64;
        sqlx::query("UPDATE chat_templates SET sort_order = ? WHERE id = ?")
            .bind(sort_order)
            .bind(&template_id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                log::error!("[reorder_templates] SQL error: {}", e);
                e.to_string()
            })?;
    }
    Ok(())
}
