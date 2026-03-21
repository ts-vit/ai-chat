use crate::types::*;

/// Convert HTML content to Markdown using htmd crate.
pub fn convert_html(
    html: &str,
    file_name: Option<&str>,
) -> Result<ConversionResult, uni_common::UniError> {
    let markdown = htmd::convert(html)
        .map_err(|e| uni_common::UniError::Generic(format!("HTML conversion failed: {}", e)))?;

    let title = extract_html_title(html).or_else(|| {
        file_name.map(|n| {
            std::path::Path::new(n)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(n)
                .to_string()
        })
    });

    let has_tables = html.contains("<table") || html.contains("<TABLE");
    let has_images = html.contains("<img") || html.contains("<IMG");

    Ok(ConversionResult {
        markdown: markdown.trim().to_string(),
        title,
        metadata: ConversionMeta {
            original_format: "html".to_string(),
            pages: None,
            word_count: count_words(&markdown),
            has_images,
            has_tables,
            quality: ConversionQuality::High,
            python_converted: false,
        },
    })
}

/// Extract title from HTML <title> tag
fn extract_html_title(html: &str) -> Option<String> {
    // Simple regex-free extraction
    let lower = html.to_lowercase();
    let start = lower.find("<title>")?;
    let content_start = start + "<title>".len();
    let end = lower[content_start..].find("</title>")?;
    let title = html[content_start..content_start + end].trim();
    if title.is_empty() {
        None
    } else {
        Some(html_entities_decode(title))
    }
}

/// Decode common HTML entities
fn html_entities_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_html() {
        let html = "<h1>Hello</h1><p>World</p>";
        let result = convert_html(html, None).unwrap();
        assert!(result.markdown.contains("# Hello"));
        assert!(result.markdown.contains("World"));
        assert_eq!(result.metadata.quality, ConversionQuality::High);
    }

    #[test]
    fn test_convert_html_with_table() {
        let html = "<table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr></table>";
        let result = convert_html(html, None).unwrap();
        assert!(result.metadata.has_tables);
        // htmd should convert tables to markdown format
        assert!(result.markdown.contains("|"));
    }

    #[test]
    fn test_convert_html_with_links() {
        let html = r#"<a href="https://example.com">Click here</a>"#;
        let result = convert_html(html, None).unwrap();
        assert!(result.markdown.contains("[Click here]"));
        assert!(result.markdown.contains("https://example.com"));
    }

    #[test]
    fn test_extract_html_title() {
        assert_eq!(
            extract_html_title("<html><head><title>My Page</title></head></html>"),
            Some("My Page".to_string())
        );
        assert_eq!(
            extract_html_title("<html><body>no title</body></html>"),
            None
        );
        assert_eq!(extract_html_title("<title></title>"), None);
        assert_eq!(
            extract_html_title("<title>Hello &amp; World</title>"),
            Some("Hello & World".to_string())
        );
    }

    #[test]
    fn test_html_entities_decode() {
        assert_eq!(html_entities_decode("a &amp; b"), "a & b");
        assert_eq!(html_entities_decode("&lt;tag&gt;"), "<tag>");
        assert_eq!(html_entities_decode("it&#39;s"), "it's");
    }

    #[test]
    fn test_convert_html_detects_images() {
        let html = r#"<p>Text</p><img src="photo.jpg" alt="Photo">"#;
        let result = convert_html(html, None).unwrap();
        assert!(result.metadata.has_images);
    }

    #[test]
    fn test_convert_html_title_from_filename() {
        let html = "<p>No title tag</p>";
        let result = convert_html(html, Some("report.html")).unwrap();
        assert_eq!(result.title.as_deref(), Some("report"));
    }
}
