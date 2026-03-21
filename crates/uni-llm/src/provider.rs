use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Build the chat completions URL from a base URL.
/// Handles trailing slashes.
///
/// # Examples
/// - `"https://openrouter.ai/api/v1"` → `"https://openrouter.ai/api/v1/chat/completions"`
/// - `"http://localhost:11434/v1/"` → `"http://localhost:11434/v1/chat/completions"`
pub fn build_llm_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

/// Build standard LLM API headers (Authorization Bearer + Content-Type JSON).
/// If api_key is empty, Authorization header is omitted.
pub fn build_llm_headers(api_key: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        if let Ok(hv) = format!("Bearer {}", api_key).parse::<HeaderValue>() {
            headers.insert(AUTHORIZATION, hv);
        }
    }
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

    #[test]
    fn test_build_llm_url() {
        assert_eq!(
            build_llm_url("https://openrouter.ai/api/v1"),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            build_llm_url("http://localhost:11434/v1/"),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn test_build_llm_headers_with_key() {
        let headers = build_llm_headers("sk-test-123");
        assert!(headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }

    #[test]
    fn test_build_llm_headers_empty_key() {
        let headers = build_llm_headers("");
        assert!(!headers.contains_key(AUTHORIZATION));
        assert!(headers.contains_key(CONTENT_TYPE));
    }
}
