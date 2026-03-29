use std::collections::HashSet;

use crate::error::WebSearchError;
use crate::helpers::normalize_url;
use crate::types::SearchResult;

/// Combined search: DuckDuckGo first, Google fallback.
/// Deduplicates results by URL.
pub async fn search(
    client: &reqwest::Client,
    query: &str,
    num_results: usize,
) -> Result<Vec<SearchResult>, WebSearchError> {
    // Try DuckDuckGo first
    let ddg_results = crate::ddg::search(client, query, num_results).await;

    // If DDG returned enough results, use them
    if let Ok(ref results) = ddg_results {
        if results.len() >= num_results {
            return ddg_results;
        }
    }

    // Otherwise try Google as fallback
    let google_results = crate::google::search(client, query, num_results).await;

    // Merge and deduplicate by URL
    let mut seen_urls: HashSet<String> = HashSet::new();
    let mut merged: Vec<SearchResult> = Vec::new();

    if let Ok(ddg) = ddg_results {
        for r in ddg {
            let normalized = normalize_url(&r.url);
            if seen_urls.insert(normalized) {
                merged.push(r);
            }
        }
    }

    if let Ok(google) = google_results {
        for r in google {
            let normalized = normalize_url(&r.url);
            if seen_urls.insert(normalized) {
                merged.push(r);
            }
        }
    }

    if merged.is_empty() {
        return Err(WebSearchError::NoResults(
            "No search results found from any engine".to_string(),
        ));
    }

    merged.truncate(num_results);
    Ok(merged)
}
