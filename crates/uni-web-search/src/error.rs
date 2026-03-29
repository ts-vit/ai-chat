use std::fmt;

#[derive(Debug)]
pub enum WebSearchError {
    /// HTTP request error
    Http(String),
    /// API error (non-2xx status)
    Api {
        provider: String,
        status: u16,
        body: String,
    },
    /// API key missing or invalid
    ApiKeyRequired(String),
    /// Parse error (HTML scraping or JSON)
    Parse(String),
    /// No results found
    NoResults(String),
    /// Other error
    Other(String),
}

impl fmt::Display for WebSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSearchError::Http(msg) => write!(f, "HTTP error: {}", msg),
            WebSearchError::Api {
                provider,
                status,
                body,
            } => {
                write!(f, "{} API error {}: {}", provider, status, body)
            }
            WebSearchError::ApiKeyRequired(provider) => {
                write!(f, "{} API key is required", provider)
            }
            WebSearchError::Parse(msg) => write!(f, "Parse error: {}", msg),
            WebSearchError::NoResults(msg) => write!(f, "No results: {}", msg),
            WebSearchError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for WebSearchError {}

impl From<WebSearchError> for String {
    fn from(e: WebSearchError) -> Self {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_http() {
        let e = WebSearchError::Http("timeout".to_string());
        assert_eq!(e.to_string(), "HTTP error: timeout");
    }

    #[test]
    fn test_display_api() {
        let e = WebSearchError::Api {
            provider: "Tavily".to_string(),
            status: 401,
            body: "unauthorized".to_string(),
        };
        assert_eq!(e.to_string(), "Tavily API error 401: unauthorized");
    }

    #[test]
    fn test_display_api_key_required() {
        let e = WebSearchError::ApiKeyRequired("Brave".to_string());
        assert_eq!(e.to_string(), "Brave API key is required");
    }

    #[test]
    fn test_display_parse() {
        let e = WebSearchError::Parse("invalid JSON".to_string());
        assert_eq!(e.to_string(), "Parse error: invalid JSON");
    }

    #[test]
    fn test_display_no_results() {
        let e = WebSearchError::NoResults("query returned empty".to_string());
        assert_eq!(e.to_string(), "No results: query returned empty");
    }

    #[test]
    fn test_display_other() {
        let e = WebSearchError::Other("something".to_string());
        assert_eq!(e.to_string(), "something");
    }

    #[test]
    fn test_from_for_string() {
        let e = WebSearchError::Http("fail".to_string());
        let s: String = e.into();
        assert_eq!(s, "HTTP error: fail");
    }
}
