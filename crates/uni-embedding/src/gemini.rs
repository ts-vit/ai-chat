// Gemini embedding provider: gemini-embedding-exp-03-07 via API

use async_trait::async_trait;
use crate::EmbeddingProvider;

const BATCH_SIZE: usize = 100;
const MODEL: &str = "gemini-embedding-exp-03-07";
const DIMENSIONS: usize = 768;
const MAX_TOKENS: usize = 8192;
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GeminiEmbeddingProvider {
    api_key: String,
    client: reqwest::Client,
}

impl GeminiEmbeddingProvider {
    pub fn new(api_key: String, client: reqwest::Client) -> Self {
        Self { api_key, client }
    }

    async fn call_batch_api(
        &self,
        texts: &[String],
        task_type: &str,
    ) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_embeddings = Vec::with_capacity(texts.len());

        for batch in texts.chunks(BATCH_SIZE) {
            let requests: Vec<serde_json::Value> = batch
                .iter()
                .map(|text| {
                    serde_json::json!({
                        "model": format!("models/{}", MODEL),
                        "content": { "parts": [{ "text": text }] },
                        "taskType": task_type,
                        "outputDimensionality": DIMENSIONS,
                    })
                })
                .collect();

            let body = serde_json::json!({ "requests": requests });

            let url = format!(
                "{}/models/{}:batchEmbedContents?key={}",
                BASE_URL, MODEL, self.api_key
            );

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Gemini embedding request failed: {}", e))?;

            let status = response.status();
            let resp_body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

            if !status.is_success() {
                let err_msg = resp_body["error"]["message"]
                    .as_str()
                    .unwrap_or("Unknown API error");
                return Err(format!("Gemini embedding API error ({}): {}", status, err_msg));
            }

            let embeddings = resp_body["embeddings"]
                .as_array()
                .ok_or("Invalid Gemini response: missing 'embeddings' array")?;

            for item in embeddings {
                let values = item["values"]
                    .as_array()
                    .ok_or("Invalid embedding values in response")?
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect::<Vec<f32>>();
                all_embeddings.push(values);
            }
        }

        Ok(all_embeddings)
    }

    async fn call_single_api(
        &self,
        text: &str,
        task_type: &str,
    ) -> Result<Vec<f32>, String> {
        let body = serde_json::json!({
            "model": format!("models/{}", MODEL),
            "content": { "parts": [{ "text": text }] },
            "taskType": task_type,
            "outputDimensionality": DIMENSIONS,
        });

        let url = format!(
            "{}/models/{}:embedContent?key={}",
            BASE_URL, MODEL, self.api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Gemini embedding request failed: {}", e))?;

        let status = response.status();
        let resp_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

        if !status.is_success() {
            let err_msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(format!("Gemini embedding API error ({}): {}", status, err_msg));
        }

        let values = resp_body["embedding"]["values"]
            .as_array()
            .ok_or("Invalid Gemini response: missing embedding values")?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(values)
    }
}

#[async_trait]
impl EmbeddingProvider for GeminiEmbeddingProvider {
    async fn embed_documents(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        self.call_batch_api(texts, "RETRIEVAL_DOCUMENT").await
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        self.call_single_api(text, "RETRIEVAL_QUERY").await
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
