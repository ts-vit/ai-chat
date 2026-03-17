// Local ONNX embedding provider: wraps EmbeddingEngine (e5-small)

use async_trait::async_trait;
use super::embedding_engine::EmbeddingEngine;
use super::embedding_provider::EmbeddingProvider;

pub struct LocalOnnxProvider;

#[async_trait]
impl EmbeddingProvider for LocalOnnxProvider {
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let engine = EmbeddingEngine::get().ok_or("Embedding engine not available")?;
        let prefixed: Vec<String> = texts
            .iter()
            .map(|t| format!("passage: {}", t))
            .collect();
        engine.embed(&prefixed).map_err(|e| format!("Embedding failed: {}", e))
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        let engine = EmbeddingEngine::get().ok_or("Embedding engine not available")?;
        let prefixed = format!("query: {}", text);
        let results = engine
            .embed(&[prefixed])
            .map_err(|e| format!("Embedding failed: {}", e))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| "Empty embedding result".to_string())
    }

    fn dimensions(&self) -> usize {
        384
    }

    fn max_tokens(&self) -> usize {
        512
    }

    fn model_id(&self) -> &str {
        "e5-small"
    }
}
