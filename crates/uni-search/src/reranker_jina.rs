// Jina Reranker API provider

use async_trait::async_trait;

use crate::reranker::{RerankRequest, RerankResult, RerankerProvider};

pub struct JinaReranker {
    api_key: String,
    http_client: reqwest::Client,
    model: String,
}

impl JinaReranker {
    pub fn new(api_key: String, http_client: reqwest::Client) -> Self {
        Self {
            api_key,
            http_client,
            model: "jina-reranker-v2-base-multilingual".to_string(),
        }
    }
}

#[async_trait]
impl RerankerProvider for JinaReranker {
    async fn rerank(&self, request: &RerankRequest) -> Result<Vec<RerankResult>, String> {
        let body = serde_json::json!({
            "model": self.model,
            "query": request.query,
            "documents": request.documents,
            "top_n": request.top_n,
        });

        let response = self
            .http_client
            .post("https://api.jina.ai/v1/rerank")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Jina rerank request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Jina rerank error {}: {}", status, text));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Jina rerank parse error: {}", e))?;

        parse_rerank_response(&data)
    }

    fn provider_name(&self) -> &str {
        "Jina"
    }

    fn max_documents(&self) -> usize {
        1000
    }

    fn max_tokens_per_doc(&self) -> usize {
        1024
    }
}

fn parse_rerank_response(data: &serde_json::Value) -> Result<Vec<RerankResult>, String> {
    let results = data["results"]
        .as_array()
        .ok_or("Missing 'results' in Jina response")?
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
    fn test_parse_jina_response() {
        let data = serde_json::json!({
            "model": "jina-reranker-v2-base-multilingual",
            "results": [
                { "index": 2, "relevance_score": 0.93 },
                { "index": 0, "relevance_score": 0.85 },
            ],
            "usage": { "total_tokens": 1234 }
        });

        let results = parse_rerank_response(&data).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 2);
        assert!((results[0].score - 0.93).abs() < 0.01);
    }

    #[test]
    fn test_parse_jina_response_empty() {
        let data = serde_json::json!({ "results": [] });
        let results = parse_rerank_response(&data).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_jina_response_missing_results() {
        let data = serde_json::json!({ "detail": "error" });
        let result = parse_rerank_response(&data);
        assert!(result.is_err());
    }
}
