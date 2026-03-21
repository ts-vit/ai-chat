pub mod openai;
pub mod gemini;

use async_trait::async_trait;

/// Trait for embedding providers (API-based).
/// Implementations: OpenAI, Gemini.
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

/// Create an embedding provider by model id.
///
/// Supported models:
/// - "openai" / "text-embedding-3-small" → OpenAI
/// - "gemini" / "gemini-embedding" → Gemini
///
/// api_key and client are required for all providers.
pub fn create_embedding_provider(
    model_id: &str,
    api_key: &str,
    client: reqwest::Client,
) -> Result<Box<dyn EmbeddingProvider>, String> {
    if api_key.is_empty() {
        return Err(format!(
            "API key required for embedding model '{}'",
            model_id
        ));
    }

    match model_id {
        "openai" | "text-embedding-3-small" => Ok(Box::new(openai::OpenAIEmbeddingProvider::new(
            api_key.to_string(),
            client,
        ))),
        "gemini" | "gemini-embedding" => Ok(Box::new(gemini::GeminiEmbeddingProvider::new(
            api_key.to_string(),
            client,
        ))),
        _ => Err(format!(
            "Unknown embedding model: {}. Supported: openai, gemini",
            model_id
        )),
    }
}

/// Get default dimensions for an embedding model.
pub fn default_dimensions(model_id: &str) -> usize {
    match model_id {
        "openai" | "text-embedding-3-small" => 1536,
        "gemini" | "gemini-embedding" => 768,
        _ => 768, // reasonable default
    }
}

pub use openai::OpenAIEmbeddingProvider;
pub use gemini::GeminiEmbeddingProvider;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_dimensions() {
        assert_eq!(default_dimensions("openai"), 1536);
        assert_eq!(default_dimensions("text-embedding-3-small"), 1536);
        assert_eq!(default_dimensions("gemini"), 768);
        assert_eq!(default_dimensions("gemini-embedding"), 768);
        assert_eq!(default_dimensions("unknown"), 768);
    }

    #[test]
    fn test_create_provider_missing_key() {
        let client = reqwest::Client::new();
        let result = create_embedding_provider("openai", "", client);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("API key required"));
    }

    #[test]
    fn test_create_provider_unknown_model() {
        let client = reqwest::Client::new();
        let result = create_embedding_provider("unknown-model", "key", client);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Unknown embedding model"));
    }

    #[test]
    fn test_create_openai_provider() {
        let client = reqwest::Client::new();
        let provider = create_embedding_provider("openai", "sk-test", client).unwrap();
        assert_eq!(provider.dimensions(), 1536);
        assert_eq!(provider.model_id(), "text-embedding-3-small");
    }

    #[test]
    fn test_create_gemini_provider() {
        let client = reqwest::Client::new();
        let provider = create_embedding_provider("gemini", "key-test", client).unwrap();
        assert_eq!(provider.dimensions(), 768);
        assert_eq!(provider.model_id(), "gemini-embedding-exp-03-07");
    }
}
