use serde::Deserialize;

use crate::error::WebSearchError;
use crate::helpers::url_encode;
use crate::types::SearchResult;

/// Search via Brave Search API
pub async fn search(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    num_results: u32,
) -> Result<Vec<SearchResult>, WebSearchError> {
    if api_key.trim().is_empty() {
        return Err(WebSearchError::ApiKeyRequired("Brave".to_string()));
    }

    let encoded_query = url_encode(query);
    let url = format!(
        "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
        encoded_query, num_results
    );

    let resp = client
        .get(&url)
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Brave request failed: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(WebSearchError::Api {
            provider: "Brave".to_string(),
            status: status.as_u16(),
            body: text,
        });
    }

    #[derive(Deserialize)]
    struct BraveResponse {
        web: Option<BraveWebResults>,
    }
    #[derive(Deserialize)]
    struct BraveWebResults {
        results: Option<Vec<BraveResult>>,
    }
    #[derive(Deserialize)]
    struct BraveResult {
        title: Option<String>,
        url: Option<String>,
        description: Option<String>,
    }

    let data: BraveResponse = resp
        .json()
        .await
        .map_err(|e| WebSearchError::Parse(format!("Brave parse error: {}", e)))?;

    let results = data
        .web
        .and_then(|w| w.results)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|r| {
            let title = r.title.unwrap_or_default();
            let url = r.url?;
            let snippet = r.description.unwrap_or_default();
            Some(SearchResult {
                title,
                url,
                snippet,
                content: None,
            })
        })
        .collect();

    Ok(results)
}
