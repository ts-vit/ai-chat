// Cohere Rerank API provider

use async_trait::async_trait;

use crate::reranker::{RerankRequest, RerankResult, RerankerProvider};

pub struct CohereReranker {
    api_key: String,
    http_client: reqwest::Client,
    model: String,
}

impl CohereReranker {
    pub fn new(api_key: String, http_client: reqwest::Client) -> Self {
        Self {
            api_key,
            http_client,
            model: "rerank-v3.5".to_string(),
        }
    }
}

#[async_trait]
impl RerankerProvider for CohereReranker {
    async fn rerank(&self, request: &RerankRequest) -> Result<Vec<RerankResult>, String> {
        let body = serde_json::json!({
            "model": self.model,
            "query": request.query,
            "documents": request.documents,
            "top_n": request.top_n,
            "max_tokens_per_doc": 4096,
        });

        let response = self
            .http_client
            .post("https://api.cohere.com/v2/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Cohere rerank request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Cohere rerank error {}: {}", status, text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Cohere rerank parse error: {}", e))?;

        parse_rerank_response(&data)
    }

    fn provider_name(&self) -> &str {
        "Cohere"
    }

    fn max_documents(&self) -> usize {
        1000
    }

    fn max_tokens_per_doc(&self) -> usize {
        4096
    }
}

fn parse_rerank_response(data: &serde_json::Value) -> Result<Vec<RerankResult>, String> {
    let results = data["results"]
        .as_array()
        .ok_or("Missing 'results' in Cohere response")?
        .iter()
        .filter_map(|r| {
            Some(RerankResult {
                index: r["index"].as_u64()? as usize,
                score: r["relevance_score"].as_f64()? as f32,
            })
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cohere_response() {
        let data = serde_json::json!({
            "results": [
                { "index": 2, "relevance_score": 0.98 },
                { "index": 0, "relevance_score": 0.85 },
                { "index": 1, "relevance_score": 0.42 },
            ]
        });

        let results = parse_rerank_response(&data).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].index, 2);
        assert!((results[0].score - 0.98).abs() < 0.01);
        assert_eq!(results[1].index, 0);
        assert!((results[1].score - 0.85).abs() < 0.01);
        assert_eq!(results[2].index, 1);
        assert!((results[2].score - 0.42).abs() < 0.01);
    }

    #[test]
    fn test_parse_cohere_response_empty() {
        let data = serde_json::json!({ "results": [] });
        let results = parse_rerank_response(&data).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_cohere_response_missing_results() {
        let data = serde_json::json!({ "message": "error" });
        let result = parse_rerank_response(&data);
        assert!(result.is_err());
    }
}
