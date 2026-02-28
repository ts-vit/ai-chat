// Команды поиска по истории: гибридный поиск, статус индексации, удаление индекса

use std::sync::Arc;
use tauri::State;
use sqlx::sqlite::SqlitePool;

use crate::commands::embeddings::INDEXING_IN_PROGRESS;
use crate::services::embedding_engine::EmbeddingEngine;
use crate::services::fts;
use crate::services::hybrid_search::{self, SearchResult};
use crate::services::vector_store::VectorStore;

#[tauri::command]
pub async fn search_messages(
    query: String,
    pool: State<'_, SqlitePool>,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let use_hybrid = store.as_ref().as_ref().is_some() && EmbeddingEngine::get().is_some();
    if use_hybrid {
        hybrid_search::hybrid_search(
            pool.inner(),
            store.as_ref().as_ref(),
            query,
            50,
        )
        .await
    } else {
        // FTS-only fallback
        let fts_results = fts::fts_search(pool.inner(), query, 50).await?;
        let mut out = Vec::with_capacity(fts_results.len());
        for r in fts_results {
            let title: String = sqlx::query_scalar("SELECT title FROM chats WHERE id = ?")
                .bind(&r.chat_id)
                .fetch_optional(pool.inner())
                .await
                .map_err(|e| {
                    eprintln!("[search_messages] SQL error (fetch chat title): {}", e);
                    e.to_string()
                })?
                .unwrap_or_default();
            out.push(SearchResult {
                message_id: r.message_id,
                chat_id: r.chat_id,
                chat_title: title,
                content: r.content_snippet,
                role: r.role,
                timestamp: r.timestamp,
                score: 1.0_f64.max(0.0), // rank уже учтён при сортировке
                source: "fts".to_string(),
            });
        }
        Ok(out)
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingStatus {
    pub indexed: usize,
    pub total: usize,
    pub in_progress: bool,
}

#[tauri::command]
pub async fn get_indexing_status(
    pool: State<'_, SqlitePool>,
    _store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<IndexingStatus, String> {
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE role IN ('user', 'assistant')",
    )
    .fetch_one(pool.inner())
    .await
    .map_err(|e| {
        eprintln!("[get_indexing_status] SQL error (total): {}", e);
        e.to_string()
    })?;
    let indexed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages WHERE fts_indexed = 1")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("[get_indexing_status] SQL error (indexed): {}", e);
            e.to_string()
        })?;
    Ok(IndexingStatus {
        indexed: indexed.0 as usize,
        total: total.0 as usize,
        in_progress: INDEXING_IN_PROGRESS.load(std::sync::atomic::Ordering::Relaxed),
    })
}

#[tauri::command]
pub async fn delete_search_index(
    chat_id: String,
    pool: State<'_, SqlitePool>,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<(), String> {
    if let Some(s) = store.as_ref().as_ref() {
        s.delete_by_chat_id(&chat_id)
            .await
            .map_err(|e| e.to_string())?;
    }
    fts::fts_delete_chat(pool.inner(), &chat_id).await
}

#[tauri::command]
pub async fn delete_message_index(
    message_id: String,
    pool: State<'_, SqlitePool>,
    store: State<'_, Arc<Option<VectorStore>>>,
) -> Result<(), String> {
    if let Some(s) = store.as_ref().as_ref() {
        s.delete_by_message_id(&message_id)
            .await
            .map_err(|e| e.to_string())?;
    }
    fts::fts_delete_message(pool.inner(), &message_id).await?;
    let _ = sqlx::query("UPDATE messages SET fts_indexed = 0 WHERE id = ?")
        .bind(&message_id)
        .execute(pool.inner())
        .await;
    Ok(())
}
