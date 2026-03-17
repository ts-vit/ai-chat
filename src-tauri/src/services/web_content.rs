/// Fetches readable content from a URL via Jina Reader or raw HTML fallback.
pub async fn fetch_content(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    match fetch_via_jina(client, url, max_words).await {
        Ok(content) if !content.trim().is_empty() => Ok(content),
        _ => fetch_raw_fallback(client, url, max_words).await,
    }
}

async fn fetch_via_jina(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    let jina_url = format!("https://r.jina.ai/{}", url);
    let resp = client
        .get(&jina_url)
        .header("Accept", "text/markdown")
        .send()
        .await
        .map_err(|e| format!("Jina Reader error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Jina Reader returned {}", resp.status()));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| format!("Jina Reader read error: {}", e))?;

    Ok(truncate_by_words(&text, max_words))
}

async fn fetch_raw_fallback(client: &reqwest::Client, url: &str, max_words: usize) -> Result<String, String> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; AIChat/1.0)")
        .send()
        .await
        .map_err(|e| format!("Fetch error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let html = resp
        .text()
        .await
        .map_err(|e| format!("Read error: {}", e))?;

    let text = strip_html_tags(&html);
    Ok(truncate_by_words(&text, max_words))
}

pub fn strip_html_tags(html: &str) -> String {
    let re_tags = regex::Regex::new(r"<(script|style|noscript)[^>]*>[\s\S]*?</\1>").unwrap();
    let cleaned = re_tags.replace_all(html, " ");
    let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re_tag.replace_all(&cleaned, " ");
    let re_ws = regex::Regex::new(r"\s+").unwrap();
    re_ws.replace_all(&text, " ").trim().to_string()
}

fn truncate_by_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_by_words_short() {
        let text = "one two three";
        assert_eq!(truncate_by_words(text, 10), text);
    }

    #[test]
    fn test_truncate_by_words_long() {
        let text = "one two three four five six seven";
        let result = truncate_by_words(text, 3);
        assert_eq!(result, "one two three");
    }

    #[test]
    fn test_truncate_by_words_russian() {
        let text = "Привет мир это тест юникода";
        let result = truncate_by_words(text, 2);
        assert_eq!(result, "Привет мир");
    }

    #[test]
    fn test_truncate_by_words_exact_limit() {
        let text = "one two three";
        assert_eq!(truncate_by_words(text, 3), text);
    }

    #[test]
    fn test_truncate_by_words_empty() {
        assert_eq!(truncate_by_words("", 10), "");
    }
}
