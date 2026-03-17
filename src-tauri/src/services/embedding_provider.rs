// Embedding provider trait and factory for pluggable embedding backends

use async_trait::async_trait;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Embed texts for indexing (documents/passages)
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String>;

    /// Embed a single query for search
    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String>;

    /// Dimension of output vectors
    fn dimensions(&self) -> usize;

    /// Max input tokens per text
    fn max_tokens(&self) -> usize;

    /// Model identifier string
    fn model_id(&self) -> &str;
}

/// Create an embedding provider based on model_id.
/// For API providers, pass a pre-built reqwest::Client (via build_http_client).
pub fn create_embedding_provider(
    model_id: &str,
    api_key: Option<&str>,
    client: Option<reqwest::Client>,
) -> Result<Box<dyn EmbeddingProvider>, String> {
    match model_id {
        "e5-small" => Ok(Box::new(super::embedding_local::LocalOnnxProvider)),
        "openai" | "text-embedding-3-small" => {
            let key = api_key
                .filter(|k| !k.is_empty())
                .ok_or("OpenAI API key required for embedding")?;
            let client = client.ok_or("HTTP client required for OpenAI embedding")?;
            Ok(Box::new(super::embedding_openai::OpenAIEmbeddingProvider::new(
                key.to_string(),
                client,
            )))
        }
        "gemini" | "gemini-embedding" => {
            let key = api_key
                .filter(|k| !k.is_empty())
                .ok_or("Gemini API key required for embedding")?;
            let client = client.ok_or("HTTP client required for Gemini embedding")?;
            Ok(Box::new(super::embedding_gemini::GeminiEmbeddingProvider::new(
                key.to_string(),
                client,
            )))
        }
        _ => Err(format!("Unknown embedding model: {}", model_id)),
    }
}

/// Get default dimensions for an embedding model
pub fn default_dimensions(model_id: &str) -> usize {
    match model_id {
        "e5-small" => 384,
        "openai" | "text-embedding-3-small" => 1536,
        "gemini" | "gemini-embedding" => 768,
        _ => 384,
    }
}
