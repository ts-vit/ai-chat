use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: Option<String>,
}

// ── Tavily ──────────────────────────────────────────────────────────────

pub async fn search_tavily(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    num_results: u32,
) -> Result<Vec<SearchResult>, String> {
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
        .map_err(|e| format!("Tavily request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Tavily API error {}: {}", status, text));
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
        .map_err(|e| format!("Tavily parse error: {}", e))?;

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

// ── Brave ───────────────────────────────────────────────────────────────

pub async fn search_brave(
    client: &reqwest::Client,
    api_key: &str,
    query: &str,
    num_results: u32,
) -> Result<Vec<SearchResult>, String> {
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
        .map_err(|e| format!("Brave request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Brave API error {}: {}", status, text));
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
        .map_err(|e| format!("Brave parse error: {}", e))?;

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

// ── UNI Search (DuckDuckGo HTML + Google HTML fallback) ─────────────────

pub async fn search_uni(
    query: &str,
    num_results: usize,
    http_client: &reqwest::Client,
) -> Result<Vec<SearchResult>, String> {
    // Try DuckDuckGo first
    let ddg_results = search_ddg_html(query, num_results, http_client).await;

    // If DDG returned enough results, use them
    if let Ok(ref results) = ddg_results {
        if results.len() >= num_results {
            return ddg_results;
        }
    }

    // Otherwise try Google as fallback
    let google_results = search_google_html(query, num_results, http_client).await;

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
        return Err("No search results found from any engine".to_string());
    }

    merged.truncate(num_results);
    Ok(merged)
}

async fn search_ddg_html(
    query: &str,
    num_results: usize,
    http_client: &reqwest::Client,
) -> Result<Vec<SearchResult>, String> {
    let form_body = format!("q={}&b=", url_encode(query));
    let resp = http_client
        .post("https://html.duckduckgo.com/html/")
        .header("User-Agent", random_user_agent())
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Referer", "https://html.duckduckgo.com/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_body)
        .send()
        .await
        .map_err(|e| format!("DuckDuckGo request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("DuckDuckGo HTTP error: {}", status));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("DuckDuckGo read error: {}", e))?;

    let document = scraper::Html::parse_document(&html);

    // Result containers
    let result_sel =
        scraper::Selector::parse("div.result, div.web-result").map_err(|e| format!("{:?}", e))?;
    let title_sel =
        scraper::Selector::parse("a.result__a").map_err(|e| format!("{:?}", e))?;
    let snippet_sel =
        scraper::Selector::parse("a.result__snippet, .result__snippet").map_err(|e| format!("{:?}", e))?;

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

        // Extract URL from href — DDG wraps in redirect: //duckduckgo.com/l/?uddg=<encoded_url>&...
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
        return Err("DuckDuckGo returned no results".to_string());
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
        // URL-decode the value
        url_decode(encoded)
    } else if href.starts_with("http") {
        href.to_string()
    } else {
        String::new()
    }
}

async fn search_google_html(
    query: &str,
    num_results: usize,
    http_client: &reqwest::Client,
) -> Result<Vec<SearchResult>, String> {
    let encoded_query = url_encode(query);
    let url = format!(
        "https://www.google.com/search?q={}&num={}&hl=en",
        encoded_query, num_results
    );

    let resp = http_client
        .get(&url)
        .header("User-Agent", random_user_agent())
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| format!("Google request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Google HTTP error: {}", status));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Google read error: {}", e))?;

    let document = scraper::Html::parse_document(&html);

    let result_sel =
        scraper::Selector::parse("div.g").map_err(|e| format!("{:?}", e))?;
    let title_sel =
        scraper::Selector::parse("h3").map_err(|e| format!("{:?}", e))?;
    let link_sel =
        scraper::Selector::parse("a[href]").map_err(|e| format!("{:?}", e))?;
    // Google changes snippet classes — try multiple selectors
    let snippet_sel =
        scraper::Selector::parse("div.VwiC3b, span.aCOpRe, div[data-sncf] span")
            .map_err(|e| format!("{:?}", e))?;

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

        // Find the first link that points to an external URL
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
        return Err("Google returned no results".to_string());
    }

    Ok(results)
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn random_user_agent() -> &'static str {
    const USER_AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
    ];
    use rand::seq::SliceRandom;
    USER_AGENTS.choose(&mut rand::thread_rng()).unwrap()
}

fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_lowercase()
}

/// Percent-decode a URL string
fn url_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &input[i + 1..i + 3],
                16,
            ) {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&result).to_string()
}

pub fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ") + "..."
}

fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}
