/// Send a non-streaming chat completion request and return the raw JSON response.
///
/// This is a thin wrapper: builds URL, headers, sends POST, returns parsed JSON.
/// Does NOT handle streaming — use SseParser for that.
pub async fn complete(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = crate::provider::build_llm_url(base_url);
    let headers = crate::provider::build_llm_headers(api_key);

    let response = client
        .post(&url)
        .headers(headers)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("LLM returned HTTP {}: {}", status, text));
    }

    response
        .json::<serde_json::Value>()
        .await
        .map_err(|e| format!("Failed to parse LLM response: {}", e))
}

/// Extract text content from a standard OpenAI chat completion response.
/// Parses `choices[0].message.content`.
pub fn extract_content(response: &serde_json::Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Extract usage from a standard OpenAI chat completion response.
pub fn extract_usage(response: &serde_json::Value) -> Option<crate::types::Usage> {
    let usage = response.get("usage")?;
    Some(crate::types::Usage {
        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_content() {
        let resp = serde_json::json!({
            "choices": [{"message": {"content": "Hello world"}}]
        });
        assert_eq!(extract_content(&resp), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_content_empty() {
        let resp = serde_json::json!({"choices": []});
        assert_eq!(extract_content(&resp), None);
    }

    #[test]
    fn test_extract_usage() {
        let resp = serde_json::json!({
            "usage": {"prompt_tokens": 10, "completion_tokens": 20, "total_tokens": 30}
        });
        let u = extract_usage(&resp).unwrap();
        assert_eq!(u.prompt_tokens, 10);
        assert_eq!(u.completion_tokens, 20);
        assert_eq!(u.total_tokens, 30);
    }
}
