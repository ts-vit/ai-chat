use crate::error::WebSearchError;
use crate::helpers::{random_user_agent, url_encode};
use crate::types::SearchResult;

/// Search via Google HTML scraping
pub async fn search(
    client: &reqwest::Client,
    query: &str,
    num_results: usize,
) -> Result<Vec<SearchResult>, WebSearchError> {
    let encoded_query = url_encode(query);
    let url = format!(
        "https://www.google.com/search?q={}&num={}&hl=en",
        encoded_query, num_results
    );

    let resp = client
        .get(&url)
        .header("User-Agent", random_user_agent())
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("Google request failed: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(WebSearchError::Api {
            provider: "Google".to_string(),
            status: status.as_u16(),
            body: format!("HTTP error: {}", status),
        });
    }

    let html = resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("Google read error: {}", e)))?;

    let document = scraper::Html::parse_document(&html);

    let result_sel = scraper::Selector::parse("div.g")
        .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;
    let title_sel =
        scraper::Selector::parse("h3").map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;
    let link_sel = scraper::Selector::parse("a[href]")
        .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;
    let snippet_sel =
        scraper::Selector::parse("div.VwiC3b, span.aCOpRe, div[data-sncf] span")
            .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;

    let mut results = Vec::new();

    for el in document.select(&result_sel) {
        if results.len() >= num_results {
            break;
        }

        let title: String = match el.select(&title_sel).next() {
            Some(h3) => h3.text().collect::<String>().trim().to_string(),
            None => continue,
        };

        if title.is_empty() {
            continue;
        }

        let link_url = el
            .select(&link_sel)
            .filter_map(|a| a.value().attr("href"))
            .find(|href| href.starts_with("http") && !href.contains("google.com"))
            .map(|s| s.to_string());

        let url = match link_url {
            Some(u) => u,
            None => continue,
        };

        let snippet: String = el
            .select(&snippet_sel)
            .next()
            .map(|s| s.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        results.push(SearchResult {
            title,
            url,
            snippet,
            content: None,
        });
    }

    if results.is_empty() {
        return Err(WebSearchError::NoResults(
            "Google returned no results".to_string(),
        ));
    }

    Ok(results)
}
