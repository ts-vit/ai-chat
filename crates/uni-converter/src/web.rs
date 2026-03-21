use crate::html;
use crate::types::*;

/// Convert a URL to Markdown: fetch → HTML → MD.
///
/// Strategy:
/// 1. Try Jina Reader (returns Markdown directly)
/// 2. Fallback: raw HTTP fetch → htmd conversion
pub async fn convert_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<ConversionResult, uni_common::UniError> {
    // Try Jina Reader first (returns Markdown)
    match fetch_via_jina(client, url).await {
        Ok(markdown) if !markdown.trim().is_empty() => {
            let title = extract_title_from_jina_md(&markdown);
            return Ok(ConversionResult {
                markdown: markdown.clone(),
                title,
                metadata: ConversionMeta {
                    original_format: "url".to_string(),
                    pages: None,
                    word_count: count_words(&markdown),
                    has_images: markdown.contains("!["),
                    has_tables: markdown.contains("| "),
                    quality: ConversionQuality::High,
                    python_converted: false,
                },
            });
        }
        Ok(_) => {
            log::debug!("Jina Reader returned empty body for URL, falling back to raw HTML");
        }
        Err(e) => {
            log::debug!(
                "Jina Reader failed for URL ({}), falling back to raw HTML: {}",
                url,
                e
            );
        }
    }

    // Fallback: fetch raw HTML → convert via htmd
    let html_content = fetch_raw_html(client, url).await?;
    let mut result = html::convert_html(&html_content, None)?;
    result.metadata.original_format = "url".to_string();
    Ok(result)
}

async fn fetch_via_jina(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, uni_common::UniError> {
    let jina_url = format!("https://r.jina.ai/{}", url);
    let resp = client
        .get(&jina_url)
        .header("Accept", "text/markdown")
        .send()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Jina Reader error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(uni_common::UniError::Generic(format!(
            "Jina Reader returned {}",
            resp.status()
        )));
    }

    resp.text()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Jina Reader read error: {}", e)))
}

async fn fetch_raw_html(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, uni_common::UniError> {
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (compatible; UNI-Converter/1.0)")
        .send()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Fetch error: {}", e)))?;

    if !resp.status().is_success() {
        return Err(uni_common::UniError::Generic(format!(
            "HTTP {}",
            resp.status()
        )));
    }

    resp.text()
        .await
        .map_err(|e| uni_common::UniError::Generic(format!("Read error: {}", e)))
}

/// Extract title from Jina Markdown output (first # heading)
fn extract_title_from_jina_md(md: &str) -> Option<String> {
    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].trim().to_string());
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_from_jina_md() {
        assert_eq!(
            extract_title_from_jina_md("# Hello World\n\nContent"),
            Some("Hello World".to_string())
        );
        assert_eq!(extract_title_from_jina_md("No heading here"), None);
        assert_eq!(extract_title_from_jina_md(""), None);
    }

    // Note: full integration tests for convert_url require network access.
    // They can be added as #[ignore] tests that run manually.
    #[tokio::test]
    #[ignore] // Requires network
    async fn test_convert_url_integration() {
        let client = reqwest::Client::new();
        let result = convert_url(&client, "https://example.com").await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.markdown.is_empty());
        assert_eq!(result.metadata.original_format, "url");
    }
}
