// Экспорт и импорт чатов (ZIP JSON, Markdown)
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sqlx::Row;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;
use zip::write::SimpleFileOptions;
use zip::write::ZipWriter;
use zip::ZipArchive;

use crate::commands::attachments::get_attachments_dir;
use crate::models::chat::{DbChat, DbMessage};

type Pool = sqlx::SqlitePool;

const TITLE_MAX_LEN: usize = 50;
const FORBIDDEN_FILENAME: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub chats_imported: u32,
    pub messages_imported: u32,
    pub attachments_imported: u32,
}

// --- Export structures (snake_case in file) ---
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportChat {
    id: String,
    title: String,
    created_at: i64,
    updated_at: i64,
    system_prompt: Option<String>,
    provider_id: String,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportMessage {
    id: String,
    role: String,
    content: String,
    timestamp: i64,
    parent_id: Option<String>,
    model: Option<String>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    cost: Option<f64>,
    has_attachments: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SingleChatExport {
    version: u32,
    exported_at: String,
    chat: ExportChat,
    messages: Vec<ExportMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatWithMessages {
    chat: ExportChat,
    messages: Vec<ExportMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BulkChatExport {
    version: u32,
    exported_at: String,
    chats: Vec<ChatWithMessages>,
}

fn db_chat_to_export(c: &DbChat) -> ExportChat {
    ExportChat {
        id: c.id.clone(),
        title: c.title.clone(),
        created_at: c.created_at,
        updated_at: c.updated_at,
        system_prompt: c.system_prompt.clone(),
        provider_id: c.provider_id.clone(),
        model: c.model.clone(),
    }
}

fn db_message_to_export(m: &DbMessage) -> ExportMessage {
    ExportMessage {
        id: m.id.clone(),
        role: m.role.clone(),
        content: m.content.clone(),
        timestamp: m.timestamp,
        parent_id: m.parent_id.clone(),
        model: m.model.clone(),
        prompt_tokens: m.prompt_tokens,
        completion_tokens: m.completion_tokens,
        cost: m.cost,
        has_attachments: m.has_attachments,
    }
}

/// YYYY-MM-DD from Unix timestamp (UTC)
fn format_date_ymd(secs: i64) -> String {
    let days = (secs / 86400) as i64;
    let d = days + 719468; // 719468 = days from 1/1/1 to 1970-01-01 in proleptic Gregorian
    let era = (if d >= 0 { d } else { d - 146096 }) / 146097;
    let day_of_era = d - era * 146097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let y = (year_of_era + era * 400) as i32;
    let day_of_year = (day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100)) as i32;
    let mp = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * mp + 2) / 5 + 1;
    let month = mp + (if mp < 10 { 3 } else { -9 });
    let year = y + (if month <= 2 { 1 } else { 0 });
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn sanitize_filename(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if FORBIDDEN_FILENAME.contains(&c) { '_' } else { c })
        .collect();
    s.chars().take(TITLE_MAX_LEN).collect()
}

fn iso8601_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_date_ymd(secs)
}

async fn get_chat_by_id(pool: &Pool, chat_id: &str) -> Result<Option<DbChat>, String> {
    let row = sqlx::query(
        "SELECT id, title, created_at, updated_at, system_prompt, provider_id, model, folder_id FROM chats WHERE id = ?",
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("[get_chat_by_id] SQL error: {}", e);
        e.to_string()
    })?;
    let row = match row {
        Some(r) => r,
        None => return Ok(None),
    };
    Ok(Some(DbChat {
        id: row.get("id"),
        title: row.get("title"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        system_prompt: row.try_get("system_prompt").ok(),
        provider_id: row.try_get("provider_id").unwrap_or_else(|_| "openrouter".to_string()),
        model: row.try_get("model").unwrap_or_default(),
        folder_id: row.try_get("folder_id").ok(),
    }))
}

async fn get_all_chats_internal(pool: &Pool) -> Result<Vec<DbChat>, String> {
    let rows = sqlx::query(
        "SELECT id, title, created_at, updated_at, system_prompt, provider_id, model, folder_id FROM chats ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("[get_all_chats_internal] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(rows
        .into_iter()
        .map(|row| DbChat {
            id: row.get("id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            system_prompt: row.try_get("system_prompt").ok(),
            provider_id: row.try_get("provider_id").unwrap_or_else(|_| "openrouter".to_string()),
            model: row.try_get("model").unwrap_or_default(),
            folder_id: row.try_get("folder_id").ok(),
        })
        .collect())
}

async fn get_messages_internal(pool: &Pool, chat_id: &str) -> Result<Vec<DbMessage>, String> {
    let rows = sqlx::query(
        "SELECT id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments FROM messages WHERE chat_id = ? ORDER BY timestamp",
    )
    .bind(chat_id)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("[get_messages_internal] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(rows
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
        })
        .collect())
}

/// Extract paths from content JSON (blocks with "path"); returns (path_str, name for zip)
fn attachment_paths_from_content(content: &str, message_id: &str) -> Vec<(String, String)> {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    for block in arr {
        let path_str = match block.get("path").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let name = block
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                Path::new(&path_str)
                    .file_name()
                    .and_then(|p| p.to_str())
                    .unwrap_or("file")
                    .to_string()
            });
        let zip_name = format!("{}_{}", message_id, name);
        out.push((path_str, zip_name));
    }
    out
}

/// Replace paths in content JSON: old message_id -> new message_id in path.
/// Path format: "attachments/{message_id}_{filename}" (message_id is UUID, 36 chars).
fn rewrite_content_paths(content: &str, old_to_new_message: &HashMap<String, String>) -> String {
    let arr: Vec<serde_json::Value> = match serde_json::from_str(content) {
        Ok(a) => a,
        Err(_) => return content.to_string(),
    };
    let mut out = Vec::new();
    for block in arr {
        let mut b = block.clone();
        if let Some(path) = b.get("path").and_then(|v| v.as_str()) {
            let path_str = path.replace('\\', "/");
            let rest = path_str.strip_prefix("attachments/").unwrap_or(&path_str);
            if rest.len() > 37 {
                let (id_part, after) = rest.split_at(36);
                if after.starts_with('_') {
                    let old_id = id_part.to_string();
                    if let Some(new_id) = old_to_new_message.get(&old_id) {
                        let filename = &after[1..];
                        let new_path = format!("attachments/{}_{}", new_id, filename);
                        b["path"] = serde_json::Value::String(new_path);
                    }
                }
            }
        }
        out.push(b);
    }
    serde_json::to_string(&out).unwrap_or_else(|_| content.to_string())
}

/// Перезаписывает один path: подставляет new_id вместо old_id. Формат path: "attachments/{message_id}_{filename}".
fn rewrite_single_path(
    path_str: &str,
    old_to_new_message: &HashMap<String, String>,
) -> Option<String> {
    let path_str = path_str.replace('\\', "/");
    let rest = path_str.strip_prefix("attachments/").unwrap_or(&path_str);
    if rest.len() <= 37 {
        return None;
    }
    let (id_part, after) = rest.split_at(36);
    if !after.starts_with('_') {
        return None;
    }
    let old_id = id_part.to_string();
    let new_id = old_to_new_message.get(&old_id)?;
    let filename = &after[1..];
    Some(format!("attachments/{}_{}", new_id, filename))
}

/// Читает данные записи из ZIP по имени. Пробует прямой by_name, затем с обратным слэшем, затем перебор по индексу (нормализация имён).
fn get_zip_entry_data(
    archive: &mut ZipArchive<std::fs::File>,
    wanted: &str,
) -> Option<Vec<u8>> {
    let normalized = wanted.replace('\\', "/");
    if let Ok(mut e) = archive.by_name(&normalized) {
        let mut data = Vec::new();
        if e.read_to_end(&mut data).is_ok() && !data.is_empty() {
            return Some(data);
        }
    }
    let with_backslash = wanted.replace('/', "\\");
    if with_backslash != normalized {
        if let Ok(mut e) = archive.by_name(&with_backslash) {
            let mut data = Vec::new();
            if e.read_to_end(&mut data).is_ok() && !data.is_empty() {
                return Some(data);
            }
        }
    }
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            if entry.name().replace('\\', "/") == normalized {
                let mut data = Vec::new();
                if entry.read_to_end(&mut data).is_ok() && !data.is_empty() {
                    return Some(data);
                }
                break;
            }
        }
    }
    None
}

#[tauri::command]
pub async fn export_chat_json(
    app: AppHandle,
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<(), String> {
    let chat = get_chat_by_id(pool.inner(), &chat_id)
        .await?
        .ok_or_else(|| "Чат не найден".to_string())?;

    let messages = get_messages_internal(pool.inner(), &chat_id).await?;

    let default_name = format!(
        "{}_{}.zip",
        sanitize_filename(&chat.title),
        format_date_ymd(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        )
    );

    let path = tokio::task::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("ZIP", &["zip"])
                .set_file_name(&default_name)
                .blocking_save_file()
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Файл не выбран".to_string())?;

    let path_buf = path.into_path().map_err(|e| e.to_string())?;

    let export_data = SingleChatExport {
        version: 1,
        exported_at: iso8601_now(),
        chat: db_chat_to_export(&chat),
        messages: messages.iter().map(db_message_to_export).collect(),
    };
    let json_bytes = serde_json::to_vec(&export_data).map_err(|e| e.to_string())?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let file = std::fs::File::create(&path_buf).map_err(|e| e.to_string())?;
    let mut zip_writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().unix_permissions(0o644);

    zip_writer
        .start_file("chat.json", opts)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(&json_bytes).map_err(|e| e.to_string())?;

    for msg in &messages {
        if msg.has_attachments.unwrap_or(0) == 0 {
            continue;
        }
        for (path_str, zip_name) in attachment_paths_from_content(&msg.content, &msg.id) {
            let full = app_data.join(&path_str);
            if let Ok(data) = std::fs::read(&full) {
                let entry_name = format!("attachments/{}", zip_name);
                zip_writer
                    .start_file(entry_name, opts)
                    .map_err(|e| e.to_string())?;
                zip_writer.write_all(&data).map_err(|e| e.to_string())?;
            }
        }
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn export_chat_markdown(
    app: AppHandle,
    pool: State<'_, Pool>,
    chat_id: String,
) -> Result<(), String> {
    let chat = get_chat_by_id(pool.inner(), &chat_id)
        .await?
        .ok_or_else(|| "Чат не найден".to_string())?;

    let messages = get_messages_internal(pool.inner(), &chat_id).await?;

    let default_name = format!(
        "{}_{}.md",
        sanitize_filename(&chat.title),
        format_date_ymd(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        )
    );

    let path = tokio::task::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("Markdown", &["md"])
                .set_file_name(&default_name)
                .blocking_save_file()
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Файл не выбран".to_string())?;

    let path_buf = path.into_path().map_err(|e| e.to_string())?;

    let created_fmt = format_date_ymd(chat.created_at);
    let mut md = format!(
        "# {}\n\n**Модель:** {}\n**Провайдер:** {}\n**Создан:** {}\n\n---\n\n",
        chat.title,
        chat.model,
        chat.provider_id,
        created_fmt
    );

    for msg in &messages {
        md.push_str(&format!("## {}\n\n", msg.role));
        let content = msg.content.trim_start();
        if content.starts_with('[') {
            if let Ok(blocks) = serde_json::from_str::<Vec<serde_json::Value>>(content) {
                for block in blocks {
                    let ty = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match ty {
                        "image" => {
                            let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("image");
                            md.push_str(&format!("![{}]({})\n\n", name, name));
                        }
                        "file" => {
                            let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("file");
                            md.push_str(&format!("📎 {}\n\n", name));
                        }
                        "text" => {
                            if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                                md.push_str(t);
                                md.push_str("\n\n");
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                md.push_str(content);
                md.push_str("\n\n");
            }
        } else {
            md.push_str(content);
            md.push_str("\n\n");
        }
        md.push_str("---\n\n");
    }

    std::fs::write(&path_buf, md).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn export_all_chats(app: AppHandle, pool: State<'_, Pool>) -> Result<(), String> {
    let chats = get_all_chats_internal(pool.inner()).await?;

    let default_name = format!(
        "ai-chat-backup_{}.zip",
        format_date_ymd(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
        )
    );

    let path = tokio::task::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("ZIP", &["zip"])
                .set_file_name(&default_name)
                .blocking_save_file()
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Файл не выбран".to_string())?;

    let path_buf = path.into_path().map_err(|e| e.to_string())?;

    let mut chats_export: Vec<ChatWithMessages> = Vec::new();
    for c in &chats {
        let messages = get_messages_internal(pool.inner(), &c.id).await?;
        chats_export.push(ChatWithMessages {
            chat: db_chat_to_export(c),
            messages: messages.iter().map(db_message_to_export).collect(),
        });
    }

    let bulk = BulkChatExport {
        version: 1,
        exported_at: iso8601_now(),
        chats: chats_export.clone(),
    };
    let json_bytes = serde_json::to_vec(&bulk).map_err(|e| e.to_string())?;

    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let file = std::fs::File::create(&path_buf).map_err(|e| e.to_string())?;
    let mut zip_writer = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().unix_permissions(0o644);

    zip_writer
        .start_file("backup.json", opts)
        .map_err(|e| e.to_string())?;
    zip_writer.write_all(&json_bytes).map_err(|e| e.to_string())?;

    for single in &bulk.chats {
        for msg in &single.messages {
            if msg.has_attachments.unwrap_or(0) == 0 {
                continue;
            }
            for (path_str, zip_name) in attachment_paths_from_content(&msg.content, &msg.id) {
                let full = app_data.join(&path_str);
                if let Ok(data) = std::fs::read(&full) {
                    let entry_name = format!("attachments/{}", zip_name);
                    zip_writer
                        .start_file(entry_name, opts)
                        .map_err(|e| e.to_string())?;
                    zip_writer.write_all(&data).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    zip_writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn import_chats(app: AppHandle, pool: State<'_, Pool>) -> Result<ImportResult, String> {
    let path = tokio::task::spawn_blocking({
        let app = app.clone();
        move || {
            app.dialog()
                .file()
                .add_filter("ZIP", &["zip"])
                .blocking_pick_file()
        }
    })
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Файл не выбран".to_string())?;

    let path_buf = path.into_path().map_err(|e| e.to_string())?;

    let file = std::fs::File::open(&path_buf).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;

    let json_name = if archive.by_name("chat.json").is_ok() {
        "chat.json"
    } else if archive.by_name("backup.json").is_ok() {
        "backup.json"
    } else {
        return Err("В архиве нет chat.json или backup.json".to_string());
    };

    let json_str = {
        let mut json_file = archive.by_name(json_name).map_err(|e| e.to_string())?;
        let mut s = String::new();
        json_file.read_to_string(&mut s).map_err(|e| e.to_string())?;
        s
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let mut zip_entry_names = Vec::new();
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            zip_entry_names.push(entry.name().to_string());
        }
    }
    eprintln!(
        "[import_chats] ZIP entries ({}): {:?}",
        zip_entry_names.len(),
        zip_entry_names
    );

    let mut chats_imported = 0u32;
    let mut messages_imported = 0u32;
    let mut attachments_imported = 0u32;

    if json_str.contains("\"chat\":") && !json_str.contains("\"chats\":") {
        let single: SingleChatExport = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
        let (c, m, a) = import_single_chat(
            &app,
            pool.inner(),
            &single.chat,
            &single.messages,
            &mut archive,
            now,
        )
        .await?;
        chats_imported += c;
        messages_imported += m;
        attachments_imported += a;
    } else {
        let bulk: BulkChatExport = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
        for single in bulk.chats {
            let (c, m, a) = import_single_chat(
                &app,
                pool.inner(),
                &single.chat,
                &single.messages,
                &mut archive,
                now,
            )
            .await?;
            chats_imported += c;
            messages_imported += m;
            attachments_imported += a;
        }
    }

    Ok(ImportResult {
        chats_imported,
        messages_imported,
        attachments_imported,
    })
}

async fn import_single_chat(
    app: &AppHandle,
    pool: &Pool,
    chat: &ExportChat,
    messages: &[ExportMessage],
    archive: &mut ZipArchive<std::fs::File>,
    now: i64,
) -> Result<(u32, u32, u32), String> {
    let new_chat_id = Uuid::new_v4().to_string();
    let mut old_to_new_msg: HashMap<String, String> = HashMap::new();

    sqlx::query(
        "INSERT INTO chats (id, title, created_at, updated_at, system_prompt, provider_id, model) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&new_chat_id)
    .bind(&chat.title)
    .bind(chat.created_at)
    .bind(now)
    .bind(&chat.system_prompt)
    .bind(&chat.provider_id)
    .bind(&chat.model)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("[import_chats] INSERT chat: {}", e);
        e.to_string()
    })?;

    let mut attachments_count = 0u32;

    for msg in messages {
        let new_msg_id = Uuid::new_v4().to_string();
        old_to_new_msg.insert(msg.id.clone(), new_msg_id.clone());

        let parent_id = msg
            .parent_id
            .as_ref()
            .and_then(|old| old_to_new_msg.get(old).cloned());

        let content = if msg.has_attachments.unwrap_or(0) != 0 {
            eprintln!(
                "[import_chats] message {} content (before rewrite): {}",
                msg.id,
                if msg.content.len() > 200 {
                    format!("{}...", &msg.content[..200])
                } else {
                    msg.content.clone()
                }
            );
            let rewritten = rewrite_content_paths(&msg.content, &old_to_new_msg);
            eprintln!(
                "[import_chats] message {} content (after rewrite): {}",
                new_msg_id,
                if rewritten.len() > 200 {
                    format!("{}...", &rewritten[..200])
                } else {
                    rewritten.clone()
                }
            );
            rewritten
        } else {
            msg.content.clone()
        };

        sqlx::query(
            "INSERT INTO messages (id, chat_id, role, content, parent_id, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments, fts_indexed) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(&new_msg_id)
        .bind(&new_chat_id)
        .bind(&msg.role)
        .bind(&content)
        .bind(&parent_id)
        .bind(msg.timestamp)
        .bind(&msg.model)
        .bind(msg.prompt_tokens)
        .bind(msg.completion_tokens)
        .bind(msg.cost)
        .bind(msg.has_attachments.unwrap_or(0))
        .execute(pool)
        .await
        .map_err(|e| {
            eprintln!("[import_chats] INSERT message: {}", e);
            e.to_string()
        })?;

        if msg.has_attachments.unwrap_or(0) != 0 {
            let base_dir = get_attachments_dir(app)?;
            let paths_for_zip = attachment_paths_from_content(&msg.content, &msg.id);
            eprintln!(
                "[import_chats] message {} attachment path pairs (path_str, zip_name): {:?}",
                new_msg_id, paths_for_zip
            );
            for (path_str, zip_name) in paths_for_zip {
                let entry_name = format!("attachments/{}", zip_name);
                eprintln!("[import_chats] looking up ZIP entry: {:?}", entry_name);
                let data = get_zip_entry_data(archive, &entry_name)
                    .or_else(|| get_zip_entry_data(archive, &path_str));
                if let Some(data) = data {
                    let target_path = if let Some(ref target_path_relative) =
                        rewrite_single_path(&path_str, &old_to_new_msg)
                    {
                        let filename = target_path_relative
                            .strip_prefix("attachments/")
                            .unwrap_or(target_path_relative);
                        base_dir.join(filename)
                    } else {
                        let file_name = zip_name.splitn(2, '_').nth(1).unwrap_or("file");
                        let target_name = format!("{}_{}", new_msg_id, file_name);
                        base_dir.join(&target_name)
                    };
                    eprintln!(
                        "[import_chats] writing attachment to: {}",
                        target_path.display()
                    );
                    match std::fs::write(&target_path, &data) {
                        Ok(()) => {
                            attachments_count += 1;
                            eprintln!("[import_chats] attachment written OK");
                        }
                        Err(e) => {
                            eprintln!(
                                "[import_chats] attachment write FAILED path={} err={}",
                                target_path.display(),
                                e
                            );
                        }
                    }
                } else {
                    eprintln!(
                        "[import_chats] attachment NOT FOUND in ZIP: {:?} (tried path {:?})",
                        entry_name, path_str
                    );
                }
            }
        }
    }

    Ok((1, messages.len() as u32, attachments_count))
}
