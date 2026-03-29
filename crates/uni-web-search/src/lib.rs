//! uni-web-search — Web search providers and content fetching for UNI Framework
//!
//! Provides search through multiple providers (Tavily, Brave, DuckDuckGo, Google)
//! and web content extraction (Jina Reader, YouTube transcripts).

mod brave;
mod content;
mod ddg;
mod error;
mod google;
mod helpers;
mod tavily;
mod types;
mod uni;
mod youtube;

pub use content::fetch_content;
pub use error::WebSearchError;
pub use helpers::{strip_html_tags, truncate_by_words, truncate_words, url_decode, url_encode};
pub use types::{SearchProvider, SearchResult};
pub use youtube::{extract_video_id, fetch_transcript};

/// Perform a web search using the specified provider.
///
/// # Arguments
/// - `client` — configured reqwest::Client (should have proxy/timeout from uni-http)
/// - `provider` — search provider to use
/// - `query` — search query string
/// - `num_results` — maximum number of results
/// - `api_key` — API key (required for Tavily, Brave; ignored for DuckDuckGo/Google/Uni)
pub async fn search(
    client: &reqwest::Client,
    provider: SearchProvider,
    query: &str,
    num_results: u32,
    api_key: Option<&str>,
) -> Result<Vec<SearchResult>, WebSearchError> {
    match provider {
        SearchProvider::Tavily => {
            let key =
                api_key.ok_or_else(|| WebSearchError::ApiKeyRequired("Tavily".to_string()))?;
            tavily::search(client, key, query, num_results).await
        }
        SearchProvider::Brave => {
            let key =
                api_key.ok_or_else(|| WebSearchError::ApiKeyRequired("Brave".to_string()))?;
            brave::search(client, key, query, num_results).await
        }
        SearchProvider::DuckDuckGo => ddg::search(client, query, num_results as usize).await,
        SearchProvider::Google => google::search(client, query, num_results as usize).await,
        SearchProvider::Uni => uni::search(client, query, num_results as usize).await,
    }
}

/// Perform a web search and fetch content for top results.
///
/// Like `search()` but also fetches readable content for the first `fetch_top_n` results
/// using Jina Reader (with raw HTML fallback).
pub async fn search_with_content(
    client: &reqwest::Client,
    provider: SearchProvider,
    query: &str,
    num_results: u32,
    api_key: Option<&str>,
    fetch_top_n: usize,
    max_content_words: usize,
) -> Result<Vec<SearchResult>, WebSearchError> {
    let mut results = search(client, provider, query, num_results, api_key).await?;

    let top_n = results.len().min(fetch_top_n);
    for i in 0..top_n {
        // Skip if content already present (Tavily returns content)
        if results[i]
            .content
            .as_ref()
            .map_or(false, |c| !c.is_empty())
        {
            continue;
        }
        match fetch_content(client, &results[i].url, max_content_words).await {
            Ok(content) => {
                results[i].content = Some(content);
            }
            Err(e) => {
                log::debug!(
                    "[uni-web-search] Failed to fetch content for {}: {}",
                    results[i].url,
                    e
                );
            }
        }
    }

    Ok(results)
}
