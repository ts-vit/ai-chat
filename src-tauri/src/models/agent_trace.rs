use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunTrace {
    pub run_id: String,
    pub chat_id: String,
    pub model: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: String,
    pub total_steps: usize,
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub total_cost: f64,
    pub termination_reason: Option<String>,
    pub steps: Vec<AgentStepTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStepTrace {
    pub step: usize,
    pub started_at: i64,
    pub duration_ms: u64,
    pub llm_call: LlmCallTrace,
    pub tool_calls: Vec<ToolCallTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCallTrace {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: f64,
    pub duration_ms: u64,
    pub had_tool_calls: bool,
    pub response_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallTrace {
    pub tool_name: String,
    pub arguments_preview: String,
    pub result_preview: String,
    pub duration_ms: u64,
    pub is_error: bool,
    pub is_builtin: bool,
}

/// UTF-8 safe truncation to `max_chars` characters, appending "..." if truncated.
pub fn safe_truncate(s: &str, max_chars: usize) -> String {
    let mut end = 0;
    let mut count = 0;
    for (i, c) in s.char_indices() {
        if count >= max_chars {
            let mut result = s[..i].to_string();
            result.push_str("...");
            return result;
        }
        end = i + c.len_utf8();
        count += 1;
    }
    s[..end].to_string()
}

pub fn is_builtin_tool(name: &str) -> bool {
    matches!(
        name,
        "memory_save"
            | "memory_search"
            | "workspace_write"
            | "workspace_read"
            | "workspace_list"
            | "spawn_agent"
            | "check_agent"
            | "get_agent_result"
            | "cancel_agent"
            | "web_search"
            | "web_read"
            | "kb_search"
            | "task_complete"
    )
}

impl AgentRunTrace {
    pub fn new(run_id: String, chat_id: String, model: String, started_at: i64) -> Self {
        Self {
            run_id,
            chat_id,
            model,
            started_at,
            finished_at: None,
            status: "running".to_string(),
            total_steps: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_cost: 0.0,
            termination_reason: None,
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: AgentStepTrace) {
        self.total_input_tokens += step.llm_call.input_tokens;
        self.total_output_tokens += step.llm_call.output_tokens;
        self.total_cost += step.llm_call.cost;
        self.total_steps += 1;
        self.steps.push(step);
    }

    pub fn finish(&mut self, status: &str, termination_reason: Option<String>) {
        self.finished_at = Some(chrono::Utc::now().timestamp());
        self.status = status.to_string();
        self.termination_reason = termination_reason;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello...");
        assert_eq!(safe_truncate("short", 10), "short");
    }

    #[test]
    fn test_safe_truncate_utf8() {
        let russian = "Привет мир, это тестовая строка";
        let truncated = safe_truncate(russian, 10);
        assert_eq!(truncated, "Привет мир...");
        // Ensure no panic on UTF-8 boundaries
        assert!(truncated.is_ascii() || truncated.len() > 0);
    }

    #[test]
    fn test_safe_truncate_no_truncation() {
        assert_eq!(safe_truncate("abc", 3), "abc");
        assert_eq!(safe_truncate("", 5), "");
    }

    #[test]
    fn test_is_builtin_tool() {
        assert!(is_builtin_tool("memory_save"));
        assert!(is_builtin_tool("memory_search"));
        assert!(is_builtin_tool("workspace_write"));
        assert!(is_builtin_tool("workspace_read"));
        assert!(is_builtin_tool("workspace_list"));
        assert!(is_builtin_tool("spawn_agent"));
        assert!(is_builtin_tool("check_agent"));
        assert!(is_builtin_tool("get_agent_result"));
        assert!(is_builtin_tool("cancel_agent"));
        assert!(is_builtin_tool("web_search"));
        assert!(is_builtin_tool("web_read"));
        assert!(is_builtin_tool("kb_search"));
        assert!(is_builtin_tool("task_complete"));
        assert!(!is_builtin_tool("custom_mcp_tool"));
        assert!(!is_builtin_tool("some_tool"));
    }

    #[test]
    fn test_agent_run_trace_serialization() {
        let trace = AgentRunTrace {
            run_id: "test-run".into(),
            chat_id: "test-chat".into(),
            model: "gpt-4o".into(),
            started_at: 1700000000,
            finished_at: Some(1700000010),
            status: "completed".into(),
            total_steps: 2,
            total_input_tokens: 1000,
            total_output_tokens: 200,
            total_cost: 0.003,
            termination_reason: Some("task_complete".into()),
            steps: vec![AgentStepTrace {
                step: 1,
                started_at: 1700000000,
                duration_ms: 1500,
                llm_call: LlmCallTrace {
                    input_tokens: 500,
                    output_tokens: 100,
                    cost: 0.0015,
                    duration_ms: 800,
                    had_tool_calls: true,
                    response_preview: "Let me search...".into(),
                },
                tool_calls: vec![ToolCallTrace {
                    tool_name: "web_search".into(),
                    arguments_preview: r#"{"query":"rust news"}"#.into(),
                    result_preview: "Results found...".into(),
                    duration_ms: 340,
                    is_error: false,
                    is_builtin: true,
                }],
            }],
        };

        let json = serde_json::to_string(&trace).unwrap();
        let parsed: AgentRunTrace = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.run_id, "test-run");
        assert_eq!(parsed.total_steps, 2);
        assert_eq!(parsed.steps.len(), 1);
        assert_eq!(parsed.steps[0].tool_calls[0].tool_name, "web_search");
    }

    #[test]
    fn test_agent_run_trace_camel_case() {
        let trace = AgentRunTrace::new("r".into(), "c".into(), "m".into(), 0);
        let json = serde_json::to_string(&trace).unwrap();
        assert!(json.contains("\"runId\""));
        assert!(json.contains("\"chatId\""));
        assert!(json.contains("\"startedAt\""));
        assert!(json.contains("\"totalSteps\""));
        assert!(json.contains("\"totalInputTokens\""));
        assert!(json.contains("\"totalOutputTokens\""));
        assert!(json.contains("\"totalCost\""));
        assert!(json.contains("\"terminationReason\""));
        assert!(!json.contains("run_id"));
    }
}
