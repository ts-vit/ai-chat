// Hybrid search for agent memory: semantic (LanceDB) + FTS5

use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::collections::HashMap;

use crate::models::memory::{AgentMemory, AgentMemorySearchResult};
use crate::services::memory_fts;
use uni_embedding::EmbeddingProvider;
use crate::services::memory_vector_store::MemoryVectorStore;

const SEMANTIC_LIMIT: usize = 20;
const FTS_LIMIT: i64 = 20;
const COMBINED_WEIGHT_SEMANTIC: f64 = 0.6;
const COMBINED_WEIGHT_FTS: f64 = 0.4;
const SINGLE_SOURCE_FACTOR: f64 = 0.8;
const MIN_SEMANTIC_SCORE: f64 = 0.60;

fn normalize_fts_ranks(results: &[(String, f64)]) -> Vec<(String, f64)> {
    if results.is_empty() {
        return Vec::new();
    }
    let min_rank = results.iter().map(|r| r.1).fold(f64::INFINITY, f64::min);
    let max_rank = results.iter().map(|r| r.1).fold(f64::NEG_INFINITY, f64::max);
    let range = max_rank - min_rank;
    results
        .iter()
        .map(|(id, rank)| {
            let normalized = if range > 1e-9 {
                1.0 - (rank - min_rank) / range
            } else {
                1.0
            };
            (id.clone(), normalized.max(0.0).min(1.0))
        })
        .collect()
}

pub async fn search_memories(
    pool: &SqlitePool,
    memory_store: Option<&MemoryVectorStore>,
    query: &str,
    limit: usize,
    project_id: Option<&str>,
    embedding_provider: Option<&dyn EmbeddingProvider>,
) -> Result<Vec<AgentMemorySearchResult>, String> {
    let limit = limit.max(1).min(50);
    let mut scores: HashMap<String, (f64, String)> = HashMap::new(); // id -> (score, source)

    // Over-fetch when project filtering is active to compensate for post-filter loss
    let semantic_limit = if project_id.is_some() { SEMANTIC_LIMIT * 2 } else { SEMANTIC_LIMIT };
    let fts_limit = if project_id.is_some() { FTS_LIMIT * 2 } else { FTS_LIMIT };

    // 1. Semantic search
    if let (Some(store), Some(provider)) = (memory_store, embedding_provider) {
        let query_embedding = provider.embed_query(query).await?;
        let results = store
            .search(&query_embedding, semantic_limit)
            .await
            .map_err(|e| e.to_string())?;
        for (id, similarity) in results {
            scores.insert(id, (similarity as f64, "semantic".to_string()));
        }
    }

    // 2. FTS search
    let fts_results = memory_fts::memory_fts_search(pool, query, fts_limit).await?;
    let fts_normalized = normalize_fts_ranks(&fts_results);

    // 3. Combine
    for (id, fts_score) in &fts_normalized {
        if let Some((sem_score, source)) = scores.get_mut(id) {
            let combined = COMBINED_WEIGHT_SEMANTIC * *sem_score + COMBINED_WEIGHT_FTS * fts_score;
            *sem_score = combined;
            *source = "both".to_string();
        } else {
            scores.insert(id.clone(), (fts_score * SINGLE_SOURCE_FACTOR, "fts".to_string()));
        }
    }

    // Filter semantic-only entries below threshold
    scores.retain(|_, (score, source)| {
        if source == "semantic" {
            if *score < MIN_SEMANTIC_SCORE {
                return false;
            }
            *score *= SINGLE_SOURCE_FACTOR;
        }
        true
    });

    // Sort by score
    let mut scored: Vec<(String, f64)> = scores.into_iter().map(|(id, (score, _))| (id, score)).collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Load full memory records, applying project filter at SQL level
    let mut results = Vec::with_capacity(limit);
    for (id, score) in scored {
        if results.len() >= limit {
            break;
        }

        let row = if let Some(pid) = project_id {
            // Include global memories (project_id IS NULL) and project-specific memories
            sqlx::query(
                "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at FROM agent_memory WHERE id = ? AND (project_id IS NULL OR project_id = ?)"
            )
            .bind(&id)
            .bind(pid)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query(
                "SELECT id, content, category, source_chat_id, source_message_id, project_id, is_pinned, created_at, updated_at FROM agent_memory WHERE id = ?"
            )
            .bind(&id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
        };

        if let Some(row) = row {
            results.push(AgentMemorySearchResult {
                memory: AgentMemory {
                    id: row.get("id"),
                    content: row.get("content"),
                    category: row.get("category"),
                    source_chat_id: row.get("source_chat_id"),
                    source_message_id: row.get("source_message_id"),
                    project_id: row.try_get::<Option<String>, _>("project_id").ok().flatten(),
                    is_pinned: row.get::<bool, _>("is_pinned"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                },
                score,
            });
        }
    }
    Ok(results)
}
