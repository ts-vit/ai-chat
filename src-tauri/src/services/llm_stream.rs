// Shared LLM streaming service — SSE parsing + retry, used by chat.rs and agent.rs
use std::collections::HashMap;
use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::models::chat::{StreamResponse, Usage};
use crate::services::retry::{retry_http_request, RetryResult};

/// Accumulated tool call from SSE deltas
#[derive(Debug, Clone)]
pub struct ToolCallBuffer {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Event payload structs (generic, event name is configurable via prefix)
#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamPayload {
    pub content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamDonePayload {
    pub full_content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamErrorPayload {
    pub error: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamUsagePayload {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRetryPayload {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamToolCallPayload {
    pub tool_call_id: String,
    pub server_id: String,
    pub tool_name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamToolResultPayload {
    pub tool_call_id: String,
    pub result: String,
    pub is_error: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamImagePayload {
    pub message_id: String,
    pub path: String,
    pub index: u32,
}

/// Configuration for a single LLM streaming call
pub struct LlmStreamConfig<'a> {
    pub client: &'a reqwest::Client,
    pub url: &'a str,
    pub headers: HeaderMap,
    pub body: &'a serde_json::Value,
    pub cancel_token: &'a CancellationToken,
    pub event_prefix: &'a str, // "chat-stream" or "agent-stream"
    pub app: &'a AppHandle,
}

/// Result of a single LLM streaming call (one iteration)
pub struct LlmStreamResult {
    pub content: String,
    pub tool_calls: Vec<ToolCallBuffer>,
    pub usage: Option<Usage>,
    pub got_tool_calls_finish: bool,
}

/// Stream a single LLM call: HTTP request with retry → SSE parse → collect text + tool calls.
/// Emits `{prefix}` (text chunks), `{prefix}-usage`, `{prefix}-retry` events.
/// Does NOT handle tool execution or looping — that's the caller's responsibility.
pub async fn stream_llm_call(config: &LlmStreamConfig<'_>) -> Result<LlmStreamResult, String> {
    let prefix = config.event_prefix;
    let app = config.app;

    // Retry wrapper
    let app_retry = app.clone();
    let retry_prefix = format!("{}-retry", prefix);
    let response = match retry_http_request(
        config.client,
        config.url,
        config.headers.clone(),
        config.body,
        config.cancel_token,
        |info| {
            let _ = app_retry.emit(
                retry_prefix.as_str(),
                StreamRetryPayload {
                    attempt: info.attempt,
                    max_attempts: info.max_attempts,
                    delay_ms: info.delay_ms,
                },
            );
        },
    )
    .await
    {
        RetryResult::Success(r) => r,
        RetryResult::Cancelled => {
            return Err("__cancelled__".to_string());
        }
        RetryResult::NonRetryableHttp { status, body: err_body } => {
            let st = reqwest::StatusCode::from_u16(status)
                .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            log::error!("stream_llm_call: API error {}: {}", st, err_body);
            let error_prefix = format!("{}-error", prefix);
            let _ = app.emit(
                error_prefix.as_str(),
                StreamErrorPayload { error: err_body.clone() },
            );
            return Err(err_body);
        }
        RetryResult::Failed(e) => {
            let error_prefix = format!("{}-error", prefix);
            let _ = app.emit(
                error_prefix.as_str(),
                StreamErrorPayload { error: e.clone() },
            );
            return Err(e);
        }
    };

    // SSE stream parsing
    let mut stream = response.bytes_stream();
    let mut sse_buffer = String::new();
    let mut tool_call_buffers: HashMap<usize, ToolCallBuffer> = HashMap::new();
    let mut got_tool_calls_finish = false;
    let mut content = String::new();
    let mut usage: Option<Usage> = None;

    while let Some(chunk) = stream.next().await {
        if config.cancel_token.is_cancelled() {
            return Err("__cancelled__".to_string());
        }

        let chunk = chunk.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&chunk);
        sse_buffer.push_str(&text);

        let mut stream_done = false;

        while let Some(line_end) = sse_buffer.find('\n') {
            let line = sse_buffer[..line_end].trim().to_string();
            sse_buffer = sse_buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    stream_done = true;
                    break;
                }

                if let Ok(resp) = serde_json::from_str::<StreamResponse>(data) {
                    if let Some(u) = &resp.usage {
                        let usage_prefix = format!("{}-usage", prefix);
                        let _ = app.emit(
                            usage_prefix.as_str(),
                            StreamUsagePayload {
                                prompt_tokens: u.prompt_tokens,
                                completion_tokens: u.completion_tokens,
                                total_tokens: u.total_tokens,
                            },
                        );
                        usage = Some(u.clone());
                    }

                    if let Some(choice) = resp.choices.first() {
                        if let Some(c) = &choice.delta.content {
                            content.push_str(c);
                            let _ = app.emit(
                                prefix,
                                StreamPayload { content: c.clone() },
                            );
                        }

                        if let Some(ref tcs) = choice.delta.tool_calls {
                            for tc in tcs {
                                let entry = tool_call_buffers
                                    .entry(tc.index)
                                    .or_insert_with(|| ToolCallBuffer {
                                        id: String::new(),
                                        name: String::new(),
                                        arguments: String::new(),
                                    });
                                if let Some(ref id) = tc.id {
                                    entry.id.clone_from(id);
                                }
                                if let Some(ref func) = tc.function {
                                    if let Some(ref name) = func.name {
                                        entry.name.clone_from(name);
                                    }
                                    if let Some(ref args) = func.arguments {
                                        entry.arguments.push_str(args);
                                    }
                                }
                            }
                        }

                        if choice.finish_reason.as_deref() == Some("tool_calls") {
                            got_tool_calls_finish = true;
                        }
                    }
                }
            }
        }

        if stream_done {
            break;
        }
    }

    // Sort tool calls by index
    let mut sorted_tool_calls = Vec::new();
    if got_tool_calls_finish || !tool_call_buffers.is_empty() {
        let mut indices: Vec<usize> = tool_call_buffers.keys().copied().collect();
        indices.sort();
        for idx in indices {
            if let Some(buf) = tool_call_buffers.remove(&idx) {
                sorted_tool_calls.push(buf);
            }
        }
    }

    Ok(LlmStreamResult {
        content,
        tool_calls: sorted_tool_calls,
        usage,
        got_tool_calls_finish,
    })
}
