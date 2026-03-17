// OpenAI embedding provider: text-embedding-3-small via API

use async_trait::async_trait;
use super::embedding_provider::EmbeddingProvider;

const BATCH_SIZE: usize = 100;
const MODEL: &str = "text-embedding-3-small";
const DIMENSIONS: usize = 1536;
const MAX_TOKENS: usize = 8192;

pub struct OpenAIEmbeddingProvider {
    api_key: String,
    client: reqwest::Client,
}

impl OpenAIEmbeddingProvider {
    pub fn new(api_key: String, client: reqwest::Client) -> Self {
        Self { api_key, client }
    }

    async fn call_api(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_embeddings = Vec::with_capacity(texts.len());

        for batch in texts.chunks(BATCH_SIZE) {
            let body = serde_json::json!({
                "model": MODEL,
                "input": batch,
                "dimensions": DIMENSIONS,
            });

            let response = self
                .client
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("OpenAI embedding request failed: {}", e))?;

            let status = response.status();
            let resp_body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

            if !status.is_success() {
                let err_msg = resp_body["error"]["message"]
                    .as_str()
                    .unwrap_or("Unknown API error");
                return Err(format!("OpenAI embedding API error ({}): {}", status, err_msg));
            }

            let data = resp_body["data"]
                .as_array()
                .ok_or("Invalid OpenAI response: missing 'data' array")?;

            for item in data {
                let embedding = item["embedding"]
                    .as_array()
                    .ok_or("Invalid embedding in response")?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect::<Vec<f32>>();
                all_embeddings.push(embedding);
            }
        }

        Ok(all_embeddings)
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        self.call_api(texts).await
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        let results = self.call_api(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| "Empty OpenAI embedding result".to_string())
    }

    fn dimensions(&self) -> usize {
        DIMENSIONS
    }

    fn max_tokens(&self) -> usize {
        MAX_TOKENS
    }

    fn model_id(&self) -> &str {
        MODEL
    }
}
