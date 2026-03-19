// Reranker provider trait and factory for pluggable reranking backends

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<String>,
    pub top_n: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub score: f32,
}

#[async_trait]
pub trait RerankerProvider: Send + Sync {
    async fn rerank(&self, request: &RerankRequest) -> Result<Vec<RerankResult>, String>;
    fn provider_name(&self) -> &str;
    fn max_documents(&self) -> usize;
    fn max_tokens_per_doc(&self) -> usize;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RerankerType {
    None,
    Cohere,
    Jina,
}

impl Default for RerankerType {
    fn default() -> Self {
        Self::None
    }
}

pub fn create_reranker(
    reranker_type: &RerankerType,
    api_key: &str,
    http_client: &reqwest::Client,
) -> Result<Box<dyn RerankerProvider>, String> {
    match reranker_type {
        RerankerType::None => Err("Reranking is disabled".to_string()),
        RerankerType::Cohere => Ok(Box::new(
            super::reranker_cohere::CohereReranker::new(api_key.to_string(), http_client.clone()),
        )),
        RerankerType::Jina => Ok(Box::new(
            super::reranker_jina::JinaReranker::new(api_key.to_string(), http_client.clone()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranker_type_default() {
        assert_eq!(RerankerType::default(), RerankerType::None);
    }

    #[test]
    fn test_create_reranker_none_returns_error() {
        let client = reqwest::Client::new();
        let result = create_reranker(&RerankerType::None, "key", &client);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Reranking is disabled");
    }

    #[test]
    fn test_create_reranker_cohere() {
        let client = reqwest::Client::new();
        let result = create_reranker(&RerankerType::Cohere, "test-key", &client);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider_name(), "Cohere");
    }

    #[test]
    fn test_create_reranker_jina() {
        let client = reqwest::Client::new();
        let result = create_reranker(&RerankerType::Jina, "test-key", &client);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().provider_name(), "Jina");
    }
}
