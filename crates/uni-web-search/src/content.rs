use crate::error::WebSearchError;
use crate::helpers::{strip_html_tags, truncate_by_words};

/// Fetch readable content from a URL.
/// Tries Jina Reader first, falls back to raw HTML stripping.
pub async fn fetch_content(
    client: &reqwest::Client,
    url: &str,
    max_words: usize,
) -> Result<String, WebSearchError> {
    match fetch_via_jina(client, url, max_words).await {
        Ok(content) if !content.trim().is_empty() => Ok(content),
        _ => fetch_raw_fallback(client, url, max_words).await,
    }
}

async fn fetch_via_jina(
    client: &reqwest::Client,
    url: &str,
    max_words: usize,
) -> Result<String, WebSearchError> {
    let jina_url = format!("https://r.jina.ai/{}", url);
    let resp = client
        .get(&jina_url)
        .header("Accept", "text/markdown")
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Jina Reader error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(WebSearchError::Api {
            provider: "Jina Reader".to_string(),
            status: resp.status().as_u16(),
            body: format!("HTTP {}", resp.status()),
        });
    }

    let text = resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("Jina Reader read error: {}", e)))?;

    Ok(truncate_by_words(&text, max_words))
}

async fn fetch_raw_fallback(
    client: &reqwest::Client,
    url: &str,
    max_words: usize,
) -> Result<String, WebSearchError> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; AIChat/1.0)")
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Fetch error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(WebSearchError::Api {
            provider: "raw".to_string(),
            status: resp.status().as_u16(),
            body: format!("HTTP {}", resp.status()),
        });
    }

    let html = resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("Read error: {}", e)))?;

    let text = strip_html_tags(&html);
    Ok(truncate_by_words(&text, max_words))
}
