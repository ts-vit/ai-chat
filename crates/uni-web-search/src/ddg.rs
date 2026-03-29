use crate::error::WebSearchError;
use crate::helpers::{random_user_agent, url_decode, url_encode};
use crate::types::SearchResult;

/// Search via DuckDuckGo HTML scraping
pub async fn search(
    client: &reqwest::Client,
    query: &str,
    num_results: usize,
) -> Result<Vec<SearchResult>, WebSearchError> {
    let form_body = format!("q={}&b=", url_encode(query));
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header("User-Agent", random_user_agent())
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Referer", "https://html.duckduckgo.com/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await
        .map_err(|e| WebSearchError::Http(format!("DuckDuckGo request failed: {}", e)))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(WebSearchError::Api {
            provider: "DuckDuckGo".to_string(),
            status: status.as_u16(),
            body: format!("HTTP error: {}", status),
        });
    }

    let html = resp
        .text()
        .await
        .map_err(|e| WebSearchError::Http(format!("DuckDuckGo read error: {}", e)))?;

    let document = scraper::Html::parse_document(&html);

    let result_sel = scraper::Selector::parse("div.result, div.web-result")
        .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;
    let title_sel = scraper::Selector::parse("a.result__a")
        .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;
    let snippet_sel = scraper::Selector::parse("a.result__snippet, .result__snippet")
        .map_err(|e| WebSearchError::Parse(format!("{:?}", e)))?;

    let mut results = Vec::new();

    for el in document.select(&result_sel) {
        if results.len() >= num_results {
            break;
        }

        let title_el = match el.select(&title_sel).next() {
            Some(t) => t,
            None => continue,
        };

        let title: String = title_el.text().collect::<String>().trim().to_string();
        if title.is_empty() {
            continue;
        }

        let raw_href = match title_el.value().attr("href") {
            Some(h) => h,
            None => continue,
        };

        let url = extract_ddg_url(raw_href);
        if url.is_empty() || !url.starts_with("http") {
            continue;
        }

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
            "DuckDuckGo returned no results".to_string(),
        ));
    }

    Ok(results)
}

/// Extract the actual URL from DuckDuckGo's redirect link
fn extract_ddg_url(href: &str) -> String {
    // Format: //duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=...
    if let Some(pos) = href.find("uddg=") {
        let after = &href[pos + 5..];
        let encoded = match after.find('&') {
            Some(end) => &after[..end],
            None => after,
        };
        url_decode(encoded)
    } else if href.starts_with("http") {
        href.to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ddg_url_redirect() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage&rut=abc";
        let url = extract_ddg_url(href);
        assert_eq!(url, "https://example.com/page");
    }

    #[test]
    fn test_extract_ddg_url_direct() {
        let href = "https://example.com/direct";
        let url = extract_ddg_url(href);
        assert_eq!(url, "https://example.com/direct");
    }

    #[test]
    fn test_extract_ddg_url_empty() {
        let url = extract_ddg_url("/something-else");
        assert!(url.is_empty());
    }

    #[test]
    fn test_extract_ddg_url_no_ampersand() {
        let href = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com";
        let url = extract_ddg_url(href);
        assert_eq!(url, "https://example.com");
    }
}
