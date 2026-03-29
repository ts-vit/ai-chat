use serde::{Deserialize, Serialize};

/// A single search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub content: Option<String>,
}

/// Available search providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProvider {
    Tavily,
    Brave,
    DuckDuckGo,
    Google,
    /// Combined: DuckDuckGo first, Google fallback
    Uni,
}

impl SearchProvider {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tavily" => Some(Self::Tavily),
            "brave" => Some(Self::Brave),
            "duckduckgo" | "ddg" => Some(Self::DuckDuckGo),
            "google" => Some(Self::Google),
            "uni" => Some(Self::Uni),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tavily => "tavily",
            Self::Brave => "brave",
            Self::DuckDuckGo => "duckduckgo",
            Self::Google => "google",
            Self::Uni => "uni",
        }
    }

    /// Whether this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        matches!(self, Self::Tavily | Self::Brave)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_all_variants() {
        assert_eq!(SearchProvider::from_str("tavily"), Some(SearchProvider::Tavily));
        assert_eq!(SearchProvider::from_str("Brave"), Some(SearchProvider::Brave));
        assert_eq!(SearchProvider::from_str("duckduckgo"), Some(SearchProvider::DuckDuckGo));
        assert_eq!(SearchProvider::from_str("DDG"), Some(SearchProvider::DuckDuckGo));
        assert_eq!(SearchProvider::from_str("google"), Some(SearchProvider::Google));
        assert_eq!(SearchProvider::from_str("UNI"), Some(SearchProvider::Uni));
        assert_eq!(SearchProvider::from_str("unknown"), None);
    }

    #[test]
    fn test_as_str_roundtrip() {
        for provider in [
            SearchProvider::Tavily,
            SearchProvider::Brave,
            SearchProvider::DuckDuckGo,
            SearchProvider::Google,
            SearchProvider::Uni,
        ] {
            assert_eq!(SearchProvider::from_str(provider.as_str()), Some(provider));
        }
    }

    #[test]
    fn test_requires_api_key() {
        assert!(SearchProvider::Tavily.requires_api_key());
        assert!(SearchProvider::Brave.requires_api_key());
        assert!(!SearchProvider::DuckDuckGo.requires_api_key());
        assert!(!SearchProvider::Google.requires_api_key());
        assert!(!SearchProvider::Uni.requires_api_key());
    }

    #[test]
    fn test_search_result_serialize() {
        let result = SearchResult {
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            snippet: "A snippet".to_string(),
            content: Some("Full content".to_string()),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"title\":\"Test\""));
        assert!(json.contains("\"url\":\"https://example.com\""));

        let deserialized: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Test");
        assert_eq!(deserialized.content, Some("Full content".to_string()));
    }

    #[test]
    fn test_search_result_deserialize_no_content() {
        let json = r#"{"title":"T","url":"http://x","snippet":"s","content":null}"#;
        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert!(result.content.is_none());
    }
}
