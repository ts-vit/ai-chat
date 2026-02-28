// Гибридный поиск: семантика (VectorStore) + FTS5

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

use crate::services::embedding_engine::EmbeddingEngine;
use crate::services::fts::{self, FtsResult};
use crate::services::vector_store::VectorStore;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub message_id: String,
    pub chat_id: String,
    pub chat_title: String,
    pub content: String,
    pub role: String,
    pub timestamp: i64,
    pub score: f64,
    pub source: String, // "semantic" | "fts" | "both"
}

const SEMANTIC_LIMIT: usize = 30;
const FTS_LIMIT: i64 = 30;
/// Минимальный семантический score для результатов только из семантики (отсекает мусорные запросы).
const MIN_SEMANTIC_SCORE: f64 = 0.70;
const SCORE_CUTOFF: f64 = 0.50;
const COMBINED_WEIGHT_SEMANTIC: f64 = 0.6;
const COMBINED_WEIGHT_FTS: f64 = 0.4;
const SINGLE_SOURCE_FACTOR: f64 = 0.8;

/// Включить временный вывод raw scores в stderr для диагностики поиска. Включить true при отладке.
const DEBUG_SEARCH_SCORES: bool = false;

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

/// Нормализует FTS rank в 0..1 (меньший rank = лучше, инвертируем и нормализуем по выборке).
fn normalize_fts_ranks(results: &[FtsResult]) -> Vec<(String, f64)> {
    if results.is_empty() {
        return Vec::new();
    }
    let min_rank = results.iter().map(|r| r.rank).fold(f64::INFINITY, f64::min);
    let max_rank = results.iter().map(|r| r.rank).fold(f64::NEG_INFINITY, f64::max);
    let range = max_rank - min_rank;
    results
        .iter()
        .map(|r| {
            let normalized = if range > 1e-9 {
                1.0 - (r.rank - min_rank) / range
            } else {
                1.0
            };
            (r.message_id.clone(), normalized.max(0.0).min(1.0))
        })
        .collect()
}

/// Группирует семантические результаты по message_id, оставляет лучший скор и текст.
fn group_semantic_by_message(
    results: Vec<crate::services::vector_store::SearchResult>,
) -> HashMap<String, (f64, String)> {
    let mut by_msg: HashMap<String, (f32, String)> = HashMap::new();
    for r in results {
        let score = 1.0 - r.score; // r.score от LanceDB — distance (cosine), меньше = лучше
        let score = score.max(0.0).min(1.0);
        if let Some((best, _)) = by_msg.get(&r.message_id) {
            if score > *best {
                by_msg.insert(
                    r.message_id.clone(),
                    (score, r.chunk_text.clone()),
                );
            }
        } else {
            by_msg.insert(
                r.message_id.clone(),
                (score, r.chunk_text.clone()),
            );
        }
    }
    by_msg
        .into_iter()
        .map(|(k, (s, t))| (k, (s as f64, t)))
        .collect()
}

pub async fn hybrid_search(
    pool: &SqlitePool,
    vector_store: Option<&VectorStore>,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.max(1).min(100);
    let mut by_message: HashMap<String, (f64, String, String, i64, String)> = HashMap::new();

    // 1. Семантический поиск
    let semantic_map: HashMap<String, (f64, String)> = if let (Some(store), Some(engine)) =
        (vector_store, EmbeddingEngine::get())
    {
        let query_prefixed = format!("query: {}", query);
        let embeddings = engine
            .embed(&[query_prefixed])
            .map_err(|e| e.to_string())?;
        let query_embedding = embeddings
            .into_iter()
            .next()
            .ok_or_else(|| "embedding failed".to_string())?;
        let results = store
            .search(&query_embedding, SEMANTIC_LIMIT, None)
            .await
            .map_err(|e| e.to_string())?;
        if DEBUG_SEARCH_SCORES {
            eprintln!("[hybrid_search] query: {:?}", query);
            eprintln!("[hybrid_search] semantic raw (LanceDB score = distance, 1-score = similarity):");
            for (i, r) in results.iter().take(15).enumerate() {
                let sim = (1.0 - r.score as f64).max(0.0).min(1.0);
                eprintln!("  [{}] message_id={} raw_score={:.4} -> similarity={:.4}", i, r.message_id, r.score, sim);
            }
            if results.len() > 15 {
                eprintln!("  ... and {} more", results.len() - 15);
            }
        }
        group_semantic_by_message(results)
    } else {
        HashMap::new()
    };

    // 2. FTS поиск
    let fts_results = fts::fts_search(pool, query, FTS_LIMIT).await?;
    let fts_scores = normalize_fts_ranks(&fts_results);
    if DEBUG_SEARCH_SCORES {
        eprintln!("[hybrid_search] FTS: {} results", fts_results.len());
        for (i, r) in fts_results.iter().take(10).enumerate() {
            let norm = fts_scores.get(i).map(|(_, s)| *s).unwrap_or(0.0);
            eprintln!("  [{}] message_id={} rank={:.4} normalized={:.4}", i, r.message_id, r.rank, norm);
        }
        if fts_results.len() > 10 {
            eprintln!("  ... and {} more", fts_results.len() - 10);
        }
    }

    // 3. Объединение по message_id: (score, content, chat_id, timestamp, source)
    let fts_by_id: HashMap<String, (f64, String, i64)> = fts_results
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            fts_scores.get(i).map(|(_, s)| (r.message_id.clone(), (*s, r.chat_id.clone(), r.timestamp)))
        })
        .collect();

    for (msg_id, (sem_score, content)) in semantic_map {
        let content_preview = truncate_str(&content, 200);
        if let Some((fts_norm, chat_id, timestamp)) = fts_by_id.get(&msg_id) {
            let combined =
                COMBINED_WEIGHT_SEMANTIC * sem_score + COMBINED_WEIGHT_FTS * *fts_norm;
            by_message.insert(
                msg_id,
                (combined, content_preview, chat_id.clone(), *timestamp, "both".to_string()),
            );
        } else {
            // Только семантика: отсекаем слабые совпадения (мусорные запросы дают ~0.5)
            if sem_score >= MIN_SEMANTIC_SCORE {
                by_message.insert(
                    msg_id,
                    (sem_score * SINGLE_SOURCE_FACTOR, content_preview, String::new(), 0, "semantic".to_string()),
                );
            }
        }
    }

    for (i, r) in fts_results.iter().enumerate() {
        let fts_norm = fts_scores.get(i).map(|(_, s)| *s).unwrap_or(0.0);
        if !by_message.contains_key(&r.message_id) {
            let score = fts_norm * SINGLE_SOURCE_FACTOR;
            let content = truncate_str(&r.content_snippet, 200);
            by_message.insert(
                r.message_id.clone(),
                (score, content, r.chat_id.clone(), r.timestamp, "fts".to_string()),
            );
        }
    }

    if DEBUG_SEARCH_SCORES {
        let mut entries: Vec<_> = by_message
            .iter()
            .map(|(msg_id, v)| (msg_id.clone(), v.0, v.4.clone()))
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        eprintln!("[hybrid_search] combined (top 10 before cutoff):");
        for (i, (msg_id, score, source)) in entries.iter().take(10).enumerate() {
            eprintln!("  [{}] message_id={} combined={:.4} source={}", i, msg_id, score, source);
        }
    }

    // Заполнить chat_id и timestamp для записей только semantic (взять из БД или из уже имеющихся)
    let mut list: Vec<(String, f64, String, String, i64, String)> = by_message
        .into_iter()
        .filter(|(_, (score, _, _, _, _))| *score >= SCORE_CUTOFF)
        .map(|(msg_id, (score, content, chat_id, ts, source))| (msg_id, score, content, chat_id, ts, source))
        .collect();
    list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    list.truncate(limit);

    // Подгрузить chat_id и timestamp для semantic-only (если пустые) и chat_title для всех
    let mut out = Vec::with_capacity(list.len());
    for (message_id, score, content, chat_id, timestamp, source) in list {
        let (chat_id, timestamp) = if chat_id.is_empty() || timestamp == 0 {
            let row: Option<(String, i64)> = sqlx::query_as(
                "SELECT chat_id, timestamp FROM messages WHERE id = ?",
            )
            .bind(&message_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                eprintln!("[hybrid_search] SQL error (fetch chat_id/timestamp): {}", e);
                e.to_string()
            })?;
            row.unwrap_or((String::new(), 0))
        } else {
            (chat_id, timestamp)
        };
        let chat_title: String = sqlx::query_scalar("SELECT title FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                eprintln!("[hybrid_search] SQL error (fetch chat title): {}", e);
                e.to_string()
            })?
            .unwrap_or_default();
        out.push(SearchResult {
            message_id,
            chat_id,
            chat_title,
            content,
            role: String::new(), // при желании подгрузить из messages
            timestamp,
            score,
            source,
        });
    }

    // Подгрузить role для каждого из messages
    for r in &mut out {
        let row: Option<(String,)> = sqlx::query_as("SELECT role FROM messages WHERE id = ?")
            .bind(&r.message_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                eprintln!("[hybrid_search] SQL error (fetch role): {}", e);
                e.to_string()
            })?;
        if let Some((role,)) = row {
            r.role = role;
        }
    }

    Ok(out)
}
