use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub embedding_model: String,
    pub embedding_dimensions: i64,
    pub chunking_strategy: String,
    pub chunk_size: i64,
    pub chunk_overlap: i64,
    pub min_chunk_size: i64,
    pub retrieval_top_k: i64,
    pub retrieval_min_score: f64,
    pub query_rewriting_enabled: bool,
    pub query_decomposition_enabled: bool,
    pub query_max_variants: i64,
    pub reranker_type: String,
    pub reranker_overfetch_factor: i64,
    pub context_token_budget: i64,
    pub context_sentence_extraction: bool,
    pub context_redundancy_removal: bool,
    pub system_prompt: String,
    pub version: i64,
    pub status: String,
    pub document_count: i64,
    pub total_chunks: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbDocument {
    pub id: String,
    pub kb_id: String,
    pub name: String,
    pub source_type: String,
    pub source_path: Option<String>,
    pub source_url: Option<String>,
    pub mime_type: String,
    pub file_size: i64,
    pub chunk_count: i64,
    pub indexing_status: String,
    pub indexing_error: Option<String>,
    pub content_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbChunk {
    pub id: String,
    pub kb_id: String,
    pub document_id: String,
    pub content: String,
    pub chunk_index: i64,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
    pub metadata: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbStats {
    pub document_count: i64,
    pub total_chunks: i64,
    pub indexed_documents: i64,
    pub pending_documents: i64,
    pub failed_documents: i64,
    pub total_file_size: i64,
}
