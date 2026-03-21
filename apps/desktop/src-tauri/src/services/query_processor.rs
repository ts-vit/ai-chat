// Query Processing: multi-query rewriting + decomposition via LLM

use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProcessingConfig {
    pub multi_query_enabled: bool,
    pub decomposition_enabled: bool,
    pub max_variants: usize,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for QueryProcessingConfig {
    fn default() -> Self {
        Self {
            multi_query_enabled: true,
            decomposition_enabled: false,
            max_variants: 3,
            model: None,
            api_key: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    pub original: String,
    pub variants: Vec<String>,
    pub sub_questions: Vec<String>,
}

const LLM_TIMEOUT_SECS: u64 = 10;

/// Main entry point: process a user query into multiple search variants
pub async fn process_query(
    query: &str,
    config: &QueryProcessingConfig,
    http_client: &reqwest::Client,
) -> Result<ProcessedQuery, String> {
    let mut variants = vec![query.to_string()];
    let mut sub_questions = vec![];

    if !config.multi_query_enabled && !config.decomposition_enabled {
        return Ok(ProcessedQuery {
            original: query.to_string(),
            variants,
            sub_questions,
        });
    }

    let api_key = match &config.api_key {
        Some(k) if !k.is_empty() => k.clone(),
        _ => {
            return Ok(ProcessedQuery {
                original: query.to_string(),
                variants,
                sub_questions,
            });
        }
    };

    let base_url = config
        .base_url
        .as_deref()
        .unwrap_or("https://openrouter.ai/api/v1");

    let model = config
        .model
        .as_deref()
        .unwrap_or("openai/gpt-4o-mini");

    // Run rewrite + decompose in parallel
    let rewrite_fut = async {
        if config.multi_query_enabled {
            match timeout(
                Duration::from_secs(LLM_TIMEOUT_SECS),
                rewrite_query(query, config.max_variants, model, &api_key, base_url, http_client),
            )
            .await
            {
                Ok(Ok(rewrites)) => rewrites,
                Ok(Err(e)) => {
                    eprintln!("[QueryProcessor] Rewriting failed: {}", e);
                    vec![]
                }
                Err(_) => {
                    eprintln!("[QueryProcessor] Rewriting timed out");
                    vec![]
                }
            }
        } else {
            vec![]
        }
    };

    let decompose_fut = async {
        if config.decomposition_enabled {
            match timeout(
                Duration::from_secs(LLM_TIMEOUT_SECS),
                decompose_query(query, model, &api_key, base_url, http_client),
            )
            .await
            {
                Ok(Ok(questions)) => questions,
                Ok(Err(e)) => {
                    eprintln!("[QueryProcessor] Decomposition failed: {}", e);
                    vec![]
                }
                Err(_) => {
                    eprintln!("[QueryProcessor] Decomposition timed out");
                    vec![]
                }
            }
        } else {
            vec![]
        }
    };

    let (rewrites, decomposed) = tokio::join!(rewrite_fut, decompose_fut);

    if !decomposed.is_empty() {
        sub_questions = decomposed.clone();
        variants.extend(decomposed);
    }
    variants.extend(rewrites);

    variants = dedup_queries(variants);

    let max_total = 1 + config.max_variants * 2;
    variants.truncate(max_total);

    Ok(ProcessedQuery {
        original: query.to_string(),
        variants,
        sub_questions,
    })
}

async fn rewrite_query(
    query: &str,
    max_variants: usize,
    model: &str,
    api_key: &str,
    base_url: &str,
    http_client: &reqwest::Client,
) -> Result<Vec<String>, String> {
    let system_prompt = format!(
        "You are a search query optimizer. Given a user question, generate {} alternative search queries \
         that approach the topic from different angles. Each query should help find relevant documents \
         that the original query might miss.\n\n\
         Rules:\n\
         - Each query should be concise (under 20 words)\n\
         - Use different keywords and phrasings\n\
         - Cover different aspects of the question\n\
         - Keep the same language as the original query\n\
         - Respond with ONLY a JSON array of strings, nothing else\n\n\
         Example:\n\
         User: \"How do I handle authentication errors in the API?\"\n\
         Output: [\"API authentication error handling\", \"auth failure response codes troubleshooting\", \
         \"login token expired API fix\"]",
        max_variants
    );

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": query}
        ],
        "max_tokens": 200,
        "temperature": 0.7,
    });

    let content = call_llm(http_client, base_url, api_key, &body).await?;
    Ok(parse_json_string_array(&content))
}

async fn decompose_query(
    query: &str,
    model: &str,
    api_key: &str,
    base_url: &str,
    http_client: &reqwest::Client,
) -> Result<Vec<String>, String> {
    let system_prompt =
        "You are a question analyzer. If the following question is complex or multi-part, \
         decompose it into 2-3 simpler sub-questions that can be searched independently. \
         If the question is already simple, return it as-is in an array.\n\n\
         Rules:\n\
         - Each sub-question should be self-contained and searchable\n\
         - Keep sub-questions concise\n\
         - Maximum 3 sub-questions\n\
         - Keep the same language as the original question\n\
         - Respond with ONLY a JSON array of strings, nothing else\n\n\
         Example:\n\
         User: \"Compare the authentication methods in v1 and v2 of the API and explain which is more secure\"\n\
         Output: [\"API v1 authentication methods\", \"API v2 authentication methods\", \
         \"authentication security comparison v1 v2\"]\n\n\
         Example:\n\
         User: \"What is OAuth2?\"\n\
         Output: [\"What is OAuth2?\"]";

    let body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": query}
        ],
        "max_tokens": 200,
        "temperature": 0.3,
    });

    let content = call_llm(http_client, base_url, api_key, &body).await?;
    Ok(parse_json_string_array(&content))
}

async fn call_llm(
    http_client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<String, String> {
    let json = uni_llm::complete(
        http_client,
        base_url.trim_end_matches('/'),
        api_key,
        body,
    )
    .await
    .map_err(|e| format!("Query processing LLM call failed: {}", e))?;

    uni_llm::extract_content(&json).ok_or_else(|| "No content in LLM response".to_string())
}

pub fn parse_json_string_array(text: &str) -> Vec<String> {
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let Ok(arr) = serde_json::from_str::<Vec<String>>(cleaned) {
        return arr.into_iter().filter(|s| !s.trim().is_empty()).collect();
    }

    // Fallback: extract quoted strings
    let re = regex::Regex::new(r#""([^"]+)""#).unwrap();
    re.captures_iter(cleaned)
        .map(|c| c[1].to_string())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

pub fn dedup_queries(queries: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    queries
        .into_iter()
        .filter(|q| {
            let normalized = q.trim().to_lowercase();
            if normalized.is_empty() || seen.contains(&normalized) {
                false
            } else {
                seen.insert(normalized);
                true
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_string_array_clean() {
        let input = r#"["query one", "query two", "query three"]"#;
        let result = parse_json_string_array(input);
        assert_eq!(result, vec!["query one", "query two", "query three"]);
    }

    #[test]
    fn test_parse_json_string_array_with_code_fences() {
        let input = "```json\n[\"query one\", \"query two\"]\n```";
        let result = parse_json_string_array(input);
        assert_eq!(result, vec!["query one", "query two"]);
    }

    #[test]
    fn test_parse_json_string_array_fallback_regex() {
        let input = "Here are the queries:\n\"query one\"\n\"query two\"";
        let result = parse_json_string_array(input);
        assert_eq!(result, vec!["query one", "query two"]);
    }

    #[test]
    fn test_parse_json_string_array_empty() {
        let input = "[]";
        let result = parse_json_string_array(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_json_string_array_garbage() {
        let input = "I can't generate queries for this.";
        let result = parse_json_string_array(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_dedup_queries() {
        let queries = vec![
            "OAuth2 authentication".to_string(),
            "oauth2 authentication".to_string(),
            "API auth tokens".to_string(),
            "  OAuth2 authentication  ".to_string(),
        ];
        let result = dedup_queries(queries);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_dedup_queries_preserves_order() {
        let queries = vec![
            "first".to_string(),
            "second".to_string(),
            "first".to_string(),
            "third".to_string(),
        ];
        let result = dedup_queries(queries);
        assert_eq!(result, vec!["first", "second", "third"]);
    }

    #[test]
    fn test_dedup_queries_removes_empty() {
        let queries = vec![
            "valid".to_string(),
            "".to_string(),
            "  ".to_string(),
            "another".to_string(),
        ];
        let result = dedup_queries(queries);
        assert_eq!(result, vec!["valid", "another"]);
    }
}
