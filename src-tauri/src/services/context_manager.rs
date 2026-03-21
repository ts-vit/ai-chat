// Context Manager: token estimation, message trimming, and context budget management
// Applied after build_agent_messages() to keep conversation within model context limits

/// Estimate token count for a text string.
/// Conservative heuristic: ~3 chars per token for mixed English/Russian content.
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 2) / 3
}

/// Estimate total token count for an array of OpenAI-format messages.
pub fn estimate_message_tokens(messages: &[serde_json::Value]) -> usize {
    let mut total = 0;
    for msg in messages {
        // Base overhead per message: ~4 tokens (role, separators)
        total += 4;

        // Content
        if let Some(content) = msg.get("content") {
            if let Some(text) = content.as_str() {
                total += estimate_tokens(text);
            } else if let Some(arr) = content.as_array() {
                for block in arr {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        total += estimate_tokens(text);
                    }
                    if block.get("type").and_then(|t| t.as_str()) == Some("image_url") {
                        total += 300; // conservative average for images
                    }
                }
            }
        }

        // Tool calls on assistant messages
        if let Some(tool_calls) = msg.get("tool_calls").and_then(|t| t.as_array()) {
            for tc in tool_calls {
                total += 4; // tool call overhead
                if let Some(name) = tc.get("function").and_then(|f| f.get("name")).and_then(|n| n.as_str()) {
                    total += estimate_tokens(name);
                }
                if let Some(args) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                    total += estimate_tokens(args);
                }
            }
        }

        // Tool result content
        if msg.get("role").and_then(|r| r.as_str()) == Some("tool") {
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                total += estimate_tokens(content);
            }
        }
    }

    // Tool definitions schema overhead
    total += 200;

    total
}

/// Get approximate context limit for a model by name pattern matching.
pub fn get_model_context_limit(model: &str) -> usize {
    let model_lower = model.to_lowercase();

    if model_lower.contains("gpt-4o") || model_lower.contains("gpt-4-turbo") {
        128_000
    } else if model_lower.contains("gpt-4") {
        8_192
    } else if model_lower.contains("gpt-3.5") {
        16_385
    } else if model_lower.contains("claude-3-5") || model_lower.contains("claude-3.5") {
        200_000
    } else if model_lower.contains("claude-3") || model_lower.contains("claude-4") {
        200_000
    } else if model_lower.contains("gemini") {
        1_000_000
    } else if model_lower.contains("llama") || model_lower.contains("mistral") || model_lower.contains("mixtral") {
        32_768
    } else if model_lower.contains("deepseek") {
        64_000
    } else if model_lower.contains("qwen") {
        32_768
    } else {
        // Conservative default for unknown models
        32_768
    }
}

/// Max ratio of context to use for input (leave room for response)
pub const CONTEXT_USAGE_TARGET: f32 = 0.75;

/// Warn frontend at this usage level
#[allow(dead_code)]
pub const CONTEXT_USAGE_WARNING: f32 = 0.85;

pub struct TrimConfig {
    pub model_context_limit: usize,
    pub target_ratio: f32,
    pub keep_first_n: usize,
    pub keep_last_n: usize,
    pub compress_tool_results: bool,
    pub tool_result_max_chars: usize,
}

impl Default for TrimConfig {
    fn default() -> Self {
        Self {
            model_context_limit: 32_768,
            target_ratio: CONTEXT_USAGE_TARGET,
            keep_first_n: 2,
            keep_last_n: 20,
            compress_tool_results: true,
            tool_result_max_chars: 500,
        }
    }
}

pub struct TrimResult {
    pub messages: Vec<serde_json::Value>,
    pub original_count: usize,
    pub trimmed_count: usize,
    pub original_tokens: usize,
    pub final_tokens: usize,
    pub was_trimmed: bool,
    pub usage_ratio: f32,
}

/// Trim messages to fit within the model's context window.
///
/// Strategy:
/// 1. If under budget, return unchanged
/// 2. Keep first N messages (system prompt + initial user message)
/// 3. Keep last N messages (recent context)
/// 4. Middle section: compress tool results, then drop if still over budget
/// 5. Insert a marker so the LLM knows context was trimmed
pub fn trim_messages(
    messages: &[serde_json::Value],
    config: &TrimConfig,
) -> TrimResult {
    let original_count = messages.len();
    let token_budget = (config.model_context_limit as f32 * config.target_ratio) as usize;
    let original_tokens = estimate_message_tokens(messages);

    // Under budget — return as-is
    if original_tokens <= token_budget {
        return TrimResult {
            messages: messages.to_vec(),
            original_count,
            trimmed_count: original_count,
            original_tokens,
            final_tokens: original_tokens,
            was_trimmed: false,
            usage_ratio: original_tokens as f32 / config.model_context_limit as f32,
        };
    }

    let mut result = Vec::new();

    // 1. Keep first N messages (system prompt + initial user message)
    let first_n = config.keep_first_n.min(messages.len());
    result.extend_from_slice(&messages[..first_n]);

    // 2. Determine middle and tail boundaries
    let middle_start = first_n;
    let middle_end = messages.len().saturating_sub(config.keep_last_n);

    // 3. Compress middle section (tool results get truncated)
    if middle_start < middle_end && config.compress_tool_results {
        for msg in &messages[middle_start..middle_end] {
            result.push(compress_message(msg, config.tool_result_max_chars));
        }
    }

    // 4. Keep last N messages (recent context)
    let last_start = messages.len().saturating_sub(config.keep_last_n).max(middle_end);
    if last_start < messages.len() {
        result.extend_from_slice(&messages[last_start..]);
    }

    // 5. If still over budget, progressively drop from middle (oldest first)
    let mut final_tokens = estimate_message_tokens(&result);
    while final_tokens > token_budget && result.len() > first_n + config.keep_last_n.min(messages.len() - first_n) {
        // Remove the oldest message after the first N
        if result.len() > first_n {
            result.remove(first_n);
            final_tokens = estimate_message_tokens(&result);
        } else {
            break;
        }
    }

    // 6. If STILL over budget (last N alone exceeds), trim from beginning of last N
    while final_tokens > token_budget && result.len() > first_n + 2 {
        result.remove(first_n);
        final_tokens = estimate_message_tokens(&result);
    }

    // 7. Insert trimming marker
    if result.len() < original_count {
        let trimmed_msg_count = original_count - result.len();
        let marker = serde_json::json!({
            "role": "system",
            "content": format!(
                "[{} earlier messages were trimmed to fit context window. The conversation continues from recent messages below.]",
                trimmed_msg_count
            )
        });
        let insert_pos = first_n.min(result.len());
        result.insert(insert_pos, marker);
        final_tokens = estimate_message_tokens(&result);
    }

    TrimResult {
        trimmed_count: result.len(),
        messages: result,
        original_count,
        original_tokens,
        final_tokens,
        was_trimmed: true,
        usage_ratio: final_tokens as f32 / config.model_context_limit as f32,
    }
}

/// Compress a message by truncating long tool results and tool call arguments.
fn compress_message(msg: &serde_json::Value, max_tool_result_chars: usize) -> serde_json::Value {
    let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");

    match role {
        "tool" => {
            let mut compressed = msg.clone();
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                if content.len() > max_tool_result_chars {
                    let truncated = safe_truncate(content, max_tool_result_chars);
                    compressed["content"] = serde_json::Value::String(
                        format!("{}... [truncated, {} chars total]", truncated, content.len())
                    );
                }
            }
            compressed
        }
        "assistant" => {
            let mut compressed = msg.clone();
            if let Some(tool_calls) = compressed.get_mut("tool_calls").and_then(|t| t.as_array_mut()) {
                for tc in tool_calls {
                    if let Some(args) = tc.get("function").and_then(|f| f.get("arguments")).and_then(|a| a.as_str()) {
                        if args.len() > max_tool_result_chars {
                            let truncated = safe_truncate(args, max_tool_result_chars);
                            tc["function"]["arguments"] = serde_json::Value::String(
                                format!("{}...", truncated)
                            );
                        }
                    }
                }
            }
            compressed
        }
        _ => msg.clone(),
    }
}

/// UTF-8 safe string truncation — never splits a multi-byte character.
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_simple() {
        assert!(estimate_tokens("hello world") > 0);
        assert!(estimate_tokens("hello world") < 10);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        // Empty string: (0 + 2) / 3 = 0
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_message_tokens() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are helpful."}),
            serde_json::json!({"role": "user", "content": "Hello"}),
        ];
        let tokens = estimate_message_tokens(&messages);
        assert!(tokens > 200); // includes 200 tool schema overhead
        assert!(tokens < 300);
    }

    #[test]
    fn test_get_model_context_limit() {
        assert_eq!(get_model_context_limit("gpt-4o"), 128_000);
        assert_eq!(get_model_context_limit("gpt-4o-mini"), 128_000);
        assert_eq!(get_model_context_limit("claude-3-5-sonnet"), 200_000);
        assert_eq!(get_model_context_limit("claude-4-opus"), 200_000);
        assert_eq!(get_model_context_limit("gemini-1.5-pro"), 1_000_000);
        assert_eq!(get_model_context_limit("deepseek-chat"), 64_000);
        assert_eq!(get_model_context_limit("unknown-model"), 32_768);
    }

    #[test]
    fn test_trim_under_budget_returns_unchanged() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "Short."}),
            serde_json::json!({"role": "user", "content": "Hi"}),
        ];
        let config = TrimConfig { model_context_limit: 32_768, ..Default::default() };
        let result = trim_messages(&messages, &config);
        assert!(!result.was_trimmed);
        assert_eq!(result.messages.len(), 2);
    }

    #[test]
    fn test_trim_over_budget_removes_middle() {
        let mut messages = vec![
            serde_json::json!({"role": "system", "content": "System prompt."}),
            serde_json::json!({"role": "user", "content": "Initial question."}),
        ];
        for i in 0..48 {
            messages.push(serde_json::json!({
                "role": if i % 2 == 0 { "assistant" } else { "user" },
                "content": "x".repeat(500)
            }));
        }
        let config = TrimConfig {
            model_context_limit: 2000,
            keep_first_n: 2,
            keep_last_n: 4,
            ..Default::default()
        };
        let result = trim_messages(&messages, &config);
        assert!(result.was_trimmed);
        assert!(result.messages.len() < messages.len());
        // First message preserved
        assert_eq!(result.messages[0]["role"], "system");
    }

    #[test]
    fn test_trim_inserts_marker() {
        let mut messages = vec![
            serde_json::json!({"role": "system", "content": "System."}),
            serde_json::json!({"role": "user", "content": "Start."}),
        ];
        for _ in 0..30 {
            messages.push(serde_json::json!({"role": "assistant", "content": "x".repeat(500)}));
        }
        let config = TrimConfig {
            model_context_limit: 2000,
            keep_first_n: 2,
            keep_last_n: 4,
            ..Default::default()
        };
        let result = trim_messages(&messages, &config);
        let has_marker = result.messages.iter().any(|m| {
            m.get("content")
                .and_then(|c| c.as_str())
                .map(|s| s.contains("trimmed"))
                .unwrap_or(false)
        });
        assert!(has_marker);
    }

    #[test]
    fn test_compress_tool_result() {
        let msg = serde_json::json!({
            "role": "tool",
            "content": "x".repeat(2000),
            "tool_call_id": "test"
        });
        let compressed = compress_message(&msg, 500);
        let content = compressed["content"].as_str().unwrap();
        assert!(content.len() < 600);
        assert!(content.contains("truncated"));
    }

    #[test]
    fn test_compress_short_tool_result_unchanged() {
        let msg = serde_json::json!({
            "role": "tool",
            "content": "short result",
            "tool_call_id": "test"
        });
        let compressed = compress_message(&msg, 500);
        assert_eq!(compressed["content"].as_str().unwrap(), "short result");
    }

    #[test]
    fn test_safe_truncate_utf8() {
        let text = "Привет мир"; // Russian, multi-byte chars
        let truncated = safe_truncate(text, 8);
        assert!(truncated.len() <= 8);
        // Verify valid UTF-8 by trying to use it
        let _ = truncated.to_string();
    }

    #[test]
    fn test_safe_truncate_ascii() {
        let text = "hello world";
        let truncated = safe_truncate(text, 5);
        assert_eq!(truncated, "hello");
    }

    #[test]
    fn test_safe_truncate_within_limit() {
        let text = "hi";
        let truncated = safe_truncate(text, 100);
        assert_eq!(truncated, "hi");
    }

    #[test]
    fn test_trim_preserves_first_and_last() {
        let mut messages = vec![
            serde_json::json!({"role": "system", "content": "System prompt here."}),
            serde_json::json!({"role": "user", "content": "First user message."}),
        ];
        // Add 20 middle messages
        for i in 0..20 {
            messages.push(serde_json::json!({
                "role": if i % 2 == 0 { "assistant" } else { "user" },
                "content": "x".repeat(300)
            }));
        }
        // Add 4 recent messages
        for i in 0..4 {
            messages.push(serde_json::json!({
                "role": if i % 2 == 0 { "user" } else { "assistant" },
                "content": format!("recent_{}", i)
            }));
        }
        let config = TrimConfig {
            model_context_limit: 2000,
            keep_first_n: 2,
            keep_last_n: 4,
            ..Default::default()
        };
        let result = trim_messages(&messages, &config);
        assert!(result.was_trimmed);
        // First message is system
        assert_eq!(result.messages[0]["role"], "system");
        assert_eq!(result.messages[0]["content"], "System prompt here.");
        // Last message is the final recent one
        let last = result.messages.last().unwrap();
        assert_eq!(last["content"], "recent_3");
    }
}
