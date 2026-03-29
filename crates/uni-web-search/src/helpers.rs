use rand::seq::SliceRandom;

/// URL-encode a string (percent encoding for non-unreserved characters)
pub fn url_encode(input: &str) -> String {
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

/// URL-decode a percent-encoded string
pub fn url_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
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

/// Truncate text to max_words, adding "..." if truncated
pub fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ") + "..."
}

/// Truncate text to max_words (without "...")
pub fn truncate_by_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.to_string();
    }
    words[..max_words].join(" ")
}

/// Strip HTML tags from text, removing script/style/noscript content
pub fn strip_html_tags(html: &str) -> String {
    // Remove script, style, noscript blocks (regex crate doesn't support backreferences)
    let re_script =
        regex::Regex::new(r"(?i)<script[^>]*>[\s\S]*?</script>").unwrap();
    let cleaned = re_script.replace_all(html, " ");
    let re_style =
        regex::Regex::new(r"(?i)<style[^>]*>[\s\S]*?</style>").unwrap();
    let cleaned = re_style.replace_all(&cleaned, " ");
    let re_noscript =
        regex::Regex::new(r"(?i)<noscript[^>]*>[\s\S]*?</noscript>").unwrap();
    let cleaned = re_noscript.replace_all(&cleaned, " ");
    let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();
    let text = re_tag.replace_all(&cleaned, " ");
    let re_ws = regex::Regex::new(r"\s+").unwrap();
    re_ws.replace_all(&text, " ").trim().to_string()
}

/// Decode HTML entities (&amp; → &, etc.)
pub fn html_entities_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("\\n", "\n")
}

/// Normalize a URL (trim trailing slash, lowercase)
pub fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_lowercase()
}

/// Random user agent string for scraping
pub fn random_user_agent() -> &'static str {
    const USER_AGENTS: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:132.0) Gecko/20100101 Firefox/132.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
    ];
    USER_AGENTS.choose(&mut rand::thread_rng()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_basic() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a+b"), "a%2Bb");
    }

    #[test]
    fn test_url_encode_cyrillic() {
        let encoded = url_encode("привет");
        assert!(encoded.contains('%'));
        let decoded = url_decode(&encoded);
        assert_eq!(decoded, "привет");
    }

    #[test]
    fn test_url_decode_basic() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("a%2Bb"), "a+b");
    }

    #[test]
    fn test_url_encode_decode_roundtrip() {
        let original = "search query with spaces & symbols!";
        let encoded = url_encode(original);
        let decoded = url_decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_truncate_words_short() {
        assert_eq!(truncate_words("one two three", 10), "one two three");
    }

    #[test]
    fn test_truncate_words_long() {
        assert_eq!(
            truncate_words("one two three four five", 3),
            "one two three..."
        );
    }

    #[test]
    fn test_truncate_words_empty() {
        assert_eq!(truncate_words("", 5), "");
    }

    #[test]
    fn test_truncate_by_words_short() {
        assert_eq!(truncate_by_words("one two three", 10), "one two three");
    }

    #[test]
    fn test_truncate_by_words_long() {
        assert_eq!(
            truncate_by_words("one two three four five", 3),
            "one two three"
        );
    }

    #[test]
    fn test_truncate_by_words_russian() {
        assert_eq!(truncate_by_words("Привет мир это тест", 2), "Привет мир");
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

    #[test]
    fn test_strip_html_tags_basic() {
        let result = strip_html_tags("<p>Hello <b>world</b></p>");
        assert!(result.contains("Hello"));
        assert!(result.contains("world"));
        assert!(!result.contains("<"));
    }

    #[test]
    fn test_strip_html_tags_script() {
        let result = strip_html_tags("<p>Text</p><script>evil()</script><p>More</p>");
        assert!(result.contains("Text"));
        assert!(result.contains("More"));
        assert!(!result.contains("evil"));
    }

    #[test]
    fn test_html_entities_decode() {
        assert_eq!(html_entities_decode("hello &amp; world"), "hello & world");
        assert_eq!(
            html_entities_decode("&lt;b&gt;bold&lt;/b&gt;"),
            "<b>bold</b>"
        );
        assert_eq!(html_entities_decode("it&#39;s"), "it's");
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://Example.COM/"),
            "https://example.com"
        );
        assert_eq!(
            normalize_url("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_random_user_agent_not_empty() {
        let ua = random_user_agent();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));
    }
}
