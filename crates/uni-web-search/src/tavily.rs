use serde::Deserialize;

use crate::error::WebSearchError;
use crate::helpers::truncate_words;
use crate::types::SearchResult;

/// Search via Tavily API
pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    num_results: u32,
) -> Result<Vec<SearchResult>, WebSearchError> {
    if api_key.trim().is_empty() {
        return Err(WebSearchError::ApiKeyRequired("Tavily".to_string()));
    }

    let body = serde_json::json!({
        "api_key": api_key,
        "query": query,
        "search_depth": "basic",
        "max_results": num_results,
    });

    let resp = client
        .post("https://api.tavily.com/search")
        .json(&body)
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Tavily request failed: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(WebSearchError::Api {
            provider: "Tavily".to_string(),
            status: status.as_u16(),
            body: text,
        });
    }

    #[derive(Deserialize)]
    struct TavilyResponse {
        results: Option<Vec<TavilyResult>>,
    }
    #[derive(Deserialize)]
    struct TavilyResult {
        title: Option<String>,
        url: Option<String>,
        content: Option<String>,
    }

    let data: TavilyResponse = resp
        .json()
        .await
        .map_err(|e| WebSearchError::Parse(format!("Tavily parse error: {}", e)))?;

    let results = data
        .results
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| {
            let title = r.title.unwrap_or_default();
            let url = r.url?;
            let content_text = r.content.unwrap_or_default();
            Some(SearchResult {
                title,
                url,
                snippet: truncate_words(&content_text, 100),
                content: Some(content_text),
            })
        })
        .collect();

    Ok(results)
}
