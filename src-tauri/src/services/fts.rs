// FTS5 полнотекстовый поиск по сообщениям (ручная синхронизация)

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FtsResult {
    pub message_id: String,
    pub chat_id: String,
    pub content_snippet: String,
    pub rank: f64,
    pub role: String,
    pub timestamp: i64,
}

/// Извлекает текст из content: если JSON с блоками — только блоки type "text".
fn extract_text_for_fts(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with('[') {
        return content.to_string();
    }
    let parsed: Option<Vec<serde_json::Value>> = serde_json::from_str(content).ok();
    let Some(blocks) = parsed else {
        return content.to_string();
    };
    let mut parts = Vec::new();
    for block in &blocks {
        if let Some(obj) = block.as_object() {
            if obj.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                }
            }
        }
    }
    if parts.is_empty() {
        content.to_string()
    } else {
        parts.join("\n")
    }
}

/// Индексирует одно сообщение в standalone FTS5 таблицу.
pub async fn fts_index_message(
    pool: &SqlitePool,
    message_id: &str,
    content: &str,
) -> Result<(), String> {
    let text = extract_text_for_fts(content);
    if text.trim().is_empty() {
        return Ok(());
    }
    // Удалить старую запись при повторной индексации
    let _ = sqlx::query("DELETE FROM messages_fts WHERE message_id = ?")
        .bind(message_id)
        .execute(pool)
        .await;
    sqlx::query("INSERT INTO messages_fts(message_id, content) VALUES(?, ?)")
        .bind(message_id)
        .bind(&text)
        .execute(pool)
        .await
        .map_err(|e| {
            eprintln!("[fts_index_message] SQL error (insert): {}", e);
            e.to_string()
        })?;
    Ok(())
}

/// Экранирует запрос для FTS5 MATCH. Одно слово — без кавычек (поиск по термину), фраза — в кавычках.
fn fts_escape_query(q: &str) -> String {
    let q = q.trim();
    if q.is_empty() {
        return "\"\"".to_string();
    }
    let escaped = q.replace('"', "\"\"");
    if !q.contains(char::is_whitespace) {
        escaped
    } else {
        format!("\"{}\"", escaped)
    }
}

/// Обрезает строку по границе символов (безопасно для UTF-8).
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}...", &s[..idx]),
        None => s.to_string(),
    }
}

/// Полнотекстовый поиск. Возвращает сообщения с rank (меньше = лучше).
pub async fn fts_search(
    pool: &SqlitePool,
    query: &str,
    limit: i64,
) -> Result<Vec<FtsResult>, String> {
    let q = fts_escape_query(query);
    eprintln!("[fts_search] MATCH query={:?} limit={}", q, limit);
    let rows = sqlx::query(
        "SELECT m.id, m.chat_id, m.content, m.role, m.timestamp, bm25(messages_fts) AS rank
         FROM messages_fts
         JOIN messages m ON messages_fts.message_id = m.id
         WHERE messages_fts MATCH ?
         ORDER BY bm25(messages_fts)
         LIMIT ?",
    )
    .bind(&q)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("[fts_search] SQL error: {}", e);
        e.to_string()
    })?;
    eprintln!("[fts_search] result_count={}", rows.len());

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let content: String = row.try_get("content").unwrap_or_default();
        let snippet = truncate_str(&content, 300);
        results.push(FtsResult {
            message_id: row.try_get("id").unwrap_or_default(),
            chat_id: row.try_get("chat_id").unwrap_or_default(),
            content_snippet: snippet,
            rank: row.try_get::<f64, _>("rank").unwrap_or(0.0),
            role: row.try_get("role").unwrap_or_default(),
            timestamp: row.try_get("timestamp").unwrap_or(0),
        });
    }
    Ok(results)
}

/// Удаляет сообщение из FTS по message_id.
pub async fn fts_delete_message(pool: &SqlitePool, message_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM messages_fts WHERE message_id = ?")
        .bind(message_id)
        .execute(pool)
        .await
        .map_err(|e| {
            eprintln!("[fts_delete_message] SQL error: {}", e);
            e.to_string()
        })?;
    Ok(())
}

/// Удаляет из FTS все сообщения чата.
pub async fn fts_delete_chat(pool: &SqlitePool, chat_id: &str) -> Result<(), String> {
    sqlx::query(
        "DELETE FROM messages_fts WHERE message_id IN (SELECT id FROM messages WHERE chat_id = ?)",
    )
    .bind(chat_id)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("[fts_delete_chat] SQL error: {}", e);
        e.to_string()
    })?;
    Ok(())
}
