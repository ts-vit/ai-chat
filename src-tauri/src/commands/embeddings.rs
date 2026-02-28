// Индексация сообщений для поиска: эмбеддинги + FTS5

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};
use sqlx::sqlite::SqlitePool;

use crate::services::chunker::chunk_text;
use crate::services::embedding_engine::EmbeddingEngine;
use crate::services::fts;
use crate::services::vector_store::{ChunkRecord, VectorStore};

pub static INDEXING_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

const PASSAGE_PREFIX: &str = "passage: ";
const CHUNK_MAX_WORDS: usize = 300;
const CHUNK_OVERLAP: usize = 50;
const BATCH_SIZE: usize = 5;
const BATCH_PAUSE_MS: u64 = 2000;
const LOCKED_SLEEP_SECS: u64 = 3;
const RETRY_DELAYS_SECS: [u64; 3] = [1, 2, 4];

/// Повторяет операцию записи в SQLite до 3 раз при ошибках locked/busy с экспоненциальной паузой.
async fn with_retry<F, Fut, T>(mut op: F) -> Result<T, String>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    for (i, &delay_secs) in RETRY_DELAYS_SECS.iter().enumerate() {
        match op().await {
            Ok(t) => return Ok(t),
            Err(e) => {
                let retryable = e.contains("locked") || e.contains("busy") || e.contains("SQLITE_BUSY");
                if retryable && i < RETRY_DELAYS_SECS.len() - 1 {
                    log::warn!("SQLite write retry in {}s (attempt {}): {}", delay_secs, i + 1, e);
                    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                } else {
                    return Err(e);
                }
            }
        }
    }
    unreachable!()
}

fn is_locked_error(e: &str) -> bool {
    e.contains("locked") || e.contains("busy") || e.contains("SQLITE_BUSY")
}

/// Извлекает текст из content: если JSON с блоками — только блоки type "text".
fn extract_text(content: &str) -> String {
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

#[tauri::command]
pub async fn index_message(
    message_id: String,
    pool: State<'_, SqlitePool>,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<(), String> {
    let row: Option<(String, String, String, i64)> = sqlx::query_as(
        "SELECT id, chat_id, content, timestamp FROM messages WHERE id = ?",
    )
    .bind(&message_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[index_message] SQL error (fetch message): {}", e);
        e.to_string()
    })?;

    let (id, chat_id, content, timestamp) = row.ok_or_else(|| {
        eprintln!("[index_message] message not found: message_id={}", message_id);
        "message not found".to_string()
    })?;
    let role: String = sqlx::query_scalar("SELECT role FROM messages WHERE id = ?")
        .bind(&message_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[index_message] SQL error (fetch role): {}", e);
            e.to_string()
        })?
        .unwrap_or_else(|| "user".to_string());

    let text = extract_text(&content);
    if text.trim().is_empty() {
        return Ok(());
    }

    let engine = match EmbeddingEngine::get() {
        Some(e) => e,
        None => return Ok(()), // graceful: просто FTS ниже
    };

    let chunks = chunk_text(&text, CHUNK_MAX_WORDS, CHUNK_OVERLAP);
    if chunks.is_empty() {
        fts::fts_index_message(pool.inner(), &message_id, &text).await?;
        sqlx::query("UPDATE messages SET fts_indexed = 1 WHERE id = ?")
            .bind(&message_id)
            .execute(pool.inner())
            .await
            .map_err(|e| {
                eprintln!("[index_message] SQL error (update fts_indexed): {}", e);
                e.to_string()
            })?;
        return Ok(());
    }

    let prefixed: Vec<String> = chunks
        .iter()
        .map(|c| format!("{}{}", PASSAGE_PREFIX, c.text))
        .collect();
    let embeddings = engine.embed(&prefixed).map_err(|e| e.to_string())?;

    let source = "chat".to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let records: Vec<ChunkRecord> = chunks
        .iter()
        .zip(embeddings.iter())
        .enumerate()
        .map(|(i, (chunk, emb))| ChunkRecord {
            id: format!("{}_{}", message_id, i),
            message_id: id.clone(),
            chat_id: chat_id.clone(),
            chunk_text: chunk.text.clone(),
            embedding: emb.clone(),
            chunk_index: i as i32,
            role: role.clone(),
            source: source.clone(),
            created_at: timestamp.max(now),
        })
        .collect();

    if let Some(s) = store.as_ref().as_ref() {
        s.save_chunks(records).await.map_err(|e| e.to_string())?;
    }

    fts::fts_index_message(pool.inner(), &message_id, &text).await?;
    sqlx::query("UPDATE messages SET fts_indexed = 1 WHERE id = ?")
        .bind(&message_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[index_message] SQL error (update fts_indexed final): {}", e);
            e.to_string()
        })?;
    Ok(())
}

#[tauri::command]
pub async fn reindex_all(
    pool: State<'_, SqlitePool>,
    store: State<'_, Arc<Option<VectorStore>>>,
    app: AppHandle,
) -> Result<(), String> {
    if INDEXING_IN_PROGRESS.load(Ordering::Relaxed) {
        return Ok(());
    }
    let pool = pool.inner().clone();
    let store = Arc::clone(&store);
    let _ = tokio::spawn(async move {
        INDEXING_IN_PROGRESS.store(true, Ordering::Relaxed);
        let ids: Vec<String> = match sqlx::query_scalar::<_, String>(
            "SELECT id FROM messages WHERE fts_indexed = 0 AND role IN ('user', 'assistant')",
        )
        .fetch_all(&pool)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                eprintln!("reindex_all fetch ids: {}", e);
                INDEXING_IN_PROGRESS.store(false, Ordering::Relaxed);
                let _ = app.emit("indexing-done", ());
                return;
            }
        };
        let total = ids.len();
        for (i, batch) in ids.chunks(BATCH_SIZE).enumerate() {
            for id in batch {
                match index_message_impl(&pool, store.as_ref().as_ref(), id).await {
                    Ok(()) => {}
                    Err(e) => {
                        if is_locked_error(&e) {
                            log::warn!("FTS index locked for {}: {}, waiting {}s", id, e, LOCKED_SLEEP_SECS);
                            tokio::time::sleep(Duration::from_secs(LOCKED_SLEEP_SECS)).await;
                        } else {
                            log::warn!("FTS index failed for {}: {}, skipping", id, e);
                        }
                    }
                }
            }
            let indexed = ((i + 1) * BATCH_SIZE).min(total);
            let _ = app.emit("indexing-progress", serde_json::json!({ "indexed": indexed, "total": total }));
            tokio::time::sleep(Duration::from_millis(BATCH_PAUSE_MS)).await;
        }
        INDEXING_IN_PROGRESS.store(false, Ordering::Relaxed);
        let _ = app.emit("indexing-done", ());
    });
    Ok(())
}

pub(crate) async fn index_message_impl(
    pool: &SqlitePool,
    store: Option<&VectorStore>,
    message_id: &str,
) -> Result<(), String> {
    let row: Option<(String, String, String, i64)> = sqlx::query_as(
        "SELECT id, chat_id, content, timestamp FROM messages WHERE id = ?",
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("[index_message_impl] SQL error (fetch message): {}", e);
        e.to_string()
    })?;

    let (id, chat_id, content, timestamp) = row.ok_or_else(|| {
        eprintln!("[index_message_impl] message not found: message_id={}", message_id);
        "message not found".to_string()
    })?;
    let role: String = sqlx::query_scalar("SELECT role FROM messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| {
            eprintln!("[index_message_impl] SQL error (fetch role): {}", e);
            e.to_string()
        })?
        .unwrap_or_else(|| "user".to_string());

    let text = extract_text(&content);
    if text.trim().is_empty() {
        return Ok(());
    }

    let engine = match EmbeddingEngine::get() {
        Some(e) => e,
        None => {
            with_retry(|| fts::fts_index_message(pool, message_id, &text)).await?;
            with_retry(|| async move {
                sqlx::query("UPDATE messages SET fts_indexed = 1 WHERE id = ?")
                    .bind(message_id)
                    .execute(pool)
                    .await
                    .map_err(|e| {
                        eprintln!("[index_message_impl] SQL error (update fts_indexed): {}", e);
                        e.to_string()
                    })
            })
            .await?;
            return Ok(());
        }
    };

    let chunks = chunk_text(&text, CHUNK_MAX_WORDS, CHUNK_OVERLAP);
    if chunks.is_empty() {
        with_retry(|| fts::fts_index_message(pool, message_id, &text)).await?;
        with_retry(|| async move {
            sqlx::query("UPDATE messages SET fts_indexed = 1 WHERE id = ?")
                .bind(message_id)
                .execute(pool)
                .await
                .map_err(|e| {
                    eprintln!("[index_message_impl] SQL error (update fts_indexed empty chunks): {}", e);
                    e.to_string()
                })
        })
        .await?;
        return Ok(());
    }

    let prefixed: Vec<String> = chunks
        .iter()
        .map(|c| format!("{}{}", PASSAGE_PREFIX, c.text))
        .collect();
    let embeddings = engine.embed(&prefixed).map_err(|e| e.to_string())?;

    let source = "chat".to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let records: Vec<ChunkRecord> = chunks
        .iter()
        .zip(embeddings.iter())
        .enumerate()
        .map(|(i, (chunk, emb))| ChunkRecord {
            id: format!("{}_{}", message_id, i),
            message_id: id.clone(),
            chat_id: chat_id.clone(),
            chunk_text: chunk.text.clone(),
            embedding: emb.clone(),
            chunk_index: i as i32,
            role: role.clone(),
            source: source.clone(),
            created_at: timestamp.max(now),
        })
        .collect();

    if let Some(s) = store {
        s.save_chunks(records).await.map_err(|e| e.to_string())?;
    }

    with_retry(|| fts::fts_index_message(pool, message_id, &text)).await?;
    with_retry(|| async move {
        sqlx::query("UPDATE messages SET fts_indexed = 1 WHERE id = ?")
            .bind(message_id)
            .execute(pool)
            .await
            .map_err(|e| {
                eprintln!("[index_message_impl] SQL error (update fts_indexed final): {}", e);
                e.to_string()
            })
    })
    .await?;
    Ok(())
}
