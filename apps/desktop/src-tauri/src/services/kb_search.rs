// KB Hybrid Search: semantic (LanceDB) + FTS5

use serde::Serialize;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::collections::HashMap;

use uni_embedding::EmbeddingProvider;
use crate::services::kb_fts;
use crate::services::kb_vector_store::KbVectorStore;

const COMBINED_WEIGHT_SEMANTIC: f64 = 0.6;
const COMBINED_WEIGHT_FTS: f64 = 0.4;
const SINGLE_SOURCE_FACTOR: f64 = 0.8;
const MIN_SEMANTIC_SCORE: f64 = 0.70;

pub struct KbSearchConfig {
    pub top_k: usize,
    pub min_score: f32,
    pub semantic_weight: f64,
    pub fts_weight: f64,
}

impl Default for KbSearchConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            min_score: 0.3,
            semantic_weight: COMBINED_WEIGHT_SEMANTIC,
            fts_weight: COMBINED_WEIGHT_FTS,
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbSearchResultItem {
    pub chunk_id: String,
    pub document_id: String,
    pub document_name: String,
    pub content: String,
    pub chunk_index: i64,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Normalize FTS BM25 ranks to 0..1 (lower rank = better match, invert and normalize).
fn normalize_fts_ranks(results: &[kb_fts::KbFtsResult]) -> Vec<(String, f64)> {
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
            (r.chunk_id.clone(), normalized.max(0.0).min(1.0))
        })
        .collect()
}

pub async fn search_kb(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb_id: &str,
    query: &str,
    config: &KbSearchConfig,
    provider: &dyn EmbeddingProvider,
    embedding_dimensions: Option<i32>,
) -> Result<Vec<KbSearchResultItem>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let query_embedding = provider.embed_query(query).await?;
    search_kb_with_embedding(
        pool,
        kb_vector_store,
        kb_id,
        Some(query_embedding.as_slice()),
        query,
        config,
        embedding_dimensions,
    )
    .await
}

/// Search with optional pre-computed query embedding (orchestrator). `None` = FTS-only merge path.
pub async fn search_kb_with_embedding(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb_id: &str,
    query_embedding: Option<&[f32]>,
    query_text: &str,
    config: &KbSearchConfig,
    embedding_dimensions: Option<i32>,
) -> Result<Vec<KbSearchResultItem>, String> {
    if query_text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let over_fetch = config.top_k * 2;

    // 1. Semantic search with provided embedding
    let semantic_results: HashMap<String, (f64, String, i32)> = if let Some(query_embedding) =
        query_embedding
    {
        let results = kb_vector_store
            .search(kb_id, query_embedding, over_fetch, 0.0, embedding_dimensions)
            .await
            .map_err(|e| e.to_string())?;
        let mut map = HashMap::new();
        for r in results {
            let score = r.score as f64;
            map.entry(r.chunk_id.clone())
                .and_modify(|(best, _, _): &mut (f64, String, i32)| {
                    if score > *best {
                        *best = score;
                    }
                })
                .or_insert((score, r.document_id, r.chunk_index));
        }
        map
    } else {
        HashMap::new()
    };

    // 2. FTS search
    let fts_results = kb_fts::search_kb_fts(pool, kb_id, query_text, over_fetch).await.unwrap_or_default();
    let fts_scores = normalize_fts_ranks(&fts_results);
    let fts_by_id: HashMap<String, f64> = fts_scores.into_iter().collect();

    // 3. Merge results
    let mut combined: HashMap<String, f64> = HashMap::new();

    for (chunk_id, (sem_score, _, _)) in &semantic_results {
        if let Some(fts_score) = fts_by_id.get(chunk_id) {
            let score = config.semantic_weight * sem_score + config.fts_weight * fts_score;
            combined.insert(chunk_id.clone(), score);
        } else if *sem_score >= MIN_SEMANTIC_SCORE {
            combined.insert(chunk_id.clone(), sem_score * SINGLE_SOURCE_FACTOR);
        }
    }

    for (chunk_id, fts_score) in &fts_by_id {
        if !combined.contains_key(chunk_id) {
            combined.insert(chunk_id.clone(), fts_score * SINGLE_SOURCE_FACTOR);
        }
    }

    // 4. Sort, filter, take top_k
    let min_score = config.min_score as f64;
    let mut sorted: Vec<(String, f64)> = combined
        .into_iter()
        .filter(|(_, score)| *score >= min_score)
        .collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(config.top_k);

    if sorted.is_empty() {
        return Ok(Vec::new());
    }

    // 5. Enrich from SQLite: get content, document_name, metadata
    let mut results = Vec::with_capacity(sorted.len());
    for (chunk_id, score) in sorted {
        let row = sqlx::query(
            "SELECT c.content, c.chunk_index, c.document_id, c.metadata, d.name as document_name \
             FROM kb_chunks c JOIN kb_documents d ON c.document_id = d.id \
             WHERE c.id = ?",
        )
        .bind(&chunk_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

        if let Some(row) = row {
            let metadata_str: String = row.get("metadata");
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({}));
            results.push(KbSearchResultItem {
                chunk_id,
                document_id: row.get("document_id"),
                document_name: row.get("document_name"),
                content: row.get("content"),
                chunk_index: row.get("chunk_index"),
                score: score as f32,
                metadata,
            });
        }
    }

    Ok(results)
}
