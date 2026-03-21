// KB Search Orchestrator: multi-variant search with query processing and reranking

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;

use crate::services::context_optimizer::OptimizationTrace;
use uni_embedding::EmbeddingProvider;
use crate::services::kb_search::{search_kb_with_embedding, KbSearchConfig, KbSearchResultItem};
use crate::services::kb_vector_store::KbVectorStore;
use crate::services::query_processor::{process_query, QueryProcessingConfig};
use uni_search::{create_reranker, RerankRequest, RerankerType};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratedSearchResult {
    pub results: Vec<KbSearchResultItem>,
    pub query_trace: QueryTrace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryTrace {
    pub original_query: String,
    pub variants_used: Vec<String>,
    pub sub_questions: Vec<String>,
    pub total_candidates: usize,
    pub after_dedup: usize,
    pub reranker_used: Option<String>,
    pub rerank_candidates: usize,
    pub final_count: usize,
    pub optimization: Option<OptimizationTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerConfig {
    pub enabled: bool,
    pub reranker_type: RerankerType,
    pub api_key: Option<String>,
    pub overfetch_factor: usize,
}

impl Default for RerankerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            reranker_type: RerankerType::None,
            api_key: None,
            overfetch_factor: 4,
        }
    }
}

pub async fn search_kb_orchestrated(
    pool: &SqlitePool,
    kb_vector_store: &KbVectorStore,
    kb_id: &str,
    query: &str,
    search_config: &KbSearchConfig,
    query_config: &QueryProcessingConfig,
    reranker_config: &RerankerConfig,
    http_client: &reqwest::Client,
    embedding_provider: Option<&dyn EmbeddingProvider>,
    embedding_dimensions: Option<i32>,
) -> Result<OrchestratedSearchResult, String> {
    // Step 1: Process query into variants
    let processed = process_query(query, query_config, http_client).await?;

    // Step 2: Search with each variant (overfetch when reranking)
    let mut all_results: Vec<KbSearchResultItem> = vec![];

    let per_variant_top_k = if reranker_config.enabled {
        search_config.top_k * reranker_config.overfetch_factor
    } else {
        search_config.top_k * 2
    };
    let per_variant_min_score = if reranker_config.enabled {
        search_config.min_score * 0.5
    } else {
        search_config.min_score * 0.8
    };

    for variant in &processed.variants {
        let qemb_storage: Option<Vec<f32>> = if let Some(p) = embedding_provider {
            match p.embed_query(variant).await {
                Ok(emb) => Some(emb),
                Err(e) => {
                    eprintln!(
                        "[SearchOrchestrator] Embedding failed for variant '{}': {}",
                        variant, e
                    );
                    continue;
                }
            }
        } else {
            None
        };
        let qemb_ref: Option<&[f32]> = qemb_storage.as_deref();

        let per_variant_config = KbSearchConfig {
            top_k: per_variant_top_k,
            min_score: per_variant_min_score,
            semantic_weight: search_config.semantic_weight,
            fts_weight: search_config.fts_weight,
        };

        match search_kb_with_embedding(
            pool,
            kb_vector_store,
            kb_id,
            qemb_ref,
            variant,
            &per_variant_config,
            embedding_dimensions,
        )
        .await
        {
            Ok(results) => all_results.extend(results),
            Err(e) => {
                eprintln!(
                    "[SearchOrchestrator] Search failed for variant '{}': {}",
                    variant, e
                );
            }
        }
    }

    let total_candidates = all_results.len();

    // Step 3: Deduplicate by chunk_id (keep highest score)
    let deduped = dedup_search_results(all_results);
    let after_dedup = deduped.len();

    // Step 3.5: Reranking
    let mut reranker_used: Option<String> = None;
    let rerank_candidates = if reranker_config.enabled { deduped.len() } else { 0 };

    let reranked = if reranker_config.enabled && !deduped.is_empty() {
        match rerank_results(query, &deduped, reranker_config, http_client).await {
            Ok((results, provider_name)) => {
                reranker_used = Some(provider_name);
                results
            }
            Err(e) => {
                eprintln!(
                    "[SearchOrchestrator] Reranking failed, using original scores: {}",
                    e
                );
                deduped
            }
        }
    } else {
        deduped
    };

    // Step 4: Sort by score descending, take top_k
    let mut final_results = reranked;
    final_results
        .sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    final_results.truncate(search_config.top_k);

    let final_count = final_results.len();

    Ok(OrchestratedSearchResult {
        results: final_results,
        query_trace: QueryTrace {
            original_query: query.to_string(),
            variants_used: processed.variants,
            sub_questions: processed.sub_questions,
            total_candidates,
            after_dedup,
            reranker_used,
            rerank_candidates,
            final_count,
            optimization: None,
        },
    })
}

async fn rerank_results(
    query: &str,
    candidates: &[KbSearchResultItem],
    config: &RerankerConfig,
    http_client: &reqwest::Client,
) -> Result<(Vec<KbSearchResultItem>, String), String> {
    let api_key = config
        .api_key
        .as_deref()
        .ok_or("Reranker API key not configured")?;
    let reranker = create_reranker(&config.reranker_type, api_key, http_client)?;
    let provider_name = reranker.provider_name().to_string();

    let max_docs = reranker.max_documents();
    let documents: Vec<String> = candidates
        .iter()
        .take(max_docs)
        .map(|c| c.content.clone())
        .collect();

    let request = RerankRequest {
        query: query.to_string(),
        documents,
        top_n: candidates.len().min(max_docs),
    };

    let rerank_results = reranker.rerank(&request).await?;

    let reranked: Vec<KbSearchResultItem> = rerank_results
        .iter()
        .filter_map(|rr| {
            candidates.get(rr.index).map(|candidate| {
                let mut item = candidate.clone();
                item.score = rr.score;
                item
            })
        })
        .collect();

    Ok((reranked, provider_name))
}

fn dedup_search_results(results: Vec<KbSearchResultItem>) -> Vec<KbSearchResultItem> {
    let mut best: HashMap<String, KbSearchResultItem> = HashMap::new();
    for result in results {
        let entry = best
            .entry(result.chunk_id.clone())
            .or_insert_with(|| result.clone());
        if result.score > entry.score {
            *entry = result;
        }
    }
    best.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(chunk_id: &str, score: f32) -> KbSearchResultItem {
        KbSearchResultItem {
            chunk_id: chunk_id.to_string(),
            document_id: "doc1".to_string(),
            document_name: "test.md".to_string(),
            content: "test content".to_string(),
            chunk_index: 0,
            score,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_dedup_search_results_keeps_highest_score() {
        let results = vec![
            make_item("c1", 0.8),
            make_item("c1", 0.9),
            make_item("c2", 0.7),
        ];
        let deduped = dedup_search_results(results);
        assert_eq!(deduped.len(), 2);
        let c1 = deduped.iter().find(|r| r.chunk_id == "c1").unwrap();
        assert_eq!(c1.score, 0.9);
    }

    #[test]
    fn test_dedup_search_results_no_duplicates() {
        let results = vec![make_item("c1", 0.8), make_item("c2", 0.7)];
        let deduped = dedup_search_results(results);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_dedup_search_results_empty() {
        let results: Vec<KbSearchResultItem> = vec![];
        let deduped = dedup_search_results(results);
        assert!(deduped.is_empty());
    }

    #[test]
    fn test_reranker_config_default_is_disabled() {
        let config = RerankerConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.reranker_type, RerankerType::None);
        assert_eq!(config.overfetch_factor, 4);
    }
}
