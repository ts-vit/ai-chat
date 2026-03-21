// Shared LLM streaming service — SSE parsing + retry, used by chat.rs and agent.rs
use reqwest::header::HeaderMap;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use uni_http::{retry_http_request, RetryResult};
use uni_llm::{SseEvent, SseParser, ToolCallBuffer, Usage};

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

    let prefix_owned = prefix.to_string();
    let app_sse = app.clone();
    let sse_result = SseParser::parse_stream(
        response.bytes_stream(),
        config.cancel_token,
        |event| {
            match event {
                SseEvent::TextDelta(text) => {
                    let _ = app_sse.emit(
                        prefix_owned.as_str(),
                        StreamPayload {
                            content: text.clone(),
                        },
                    );
                }
                SseEvent::Usage(u) => {
                    let usage_prefix = format!("{}-usage", prefix_owned);
                    let _ = app_sse.emit(
                        usage_prefix.as_str(),
                        StreamUsagePayload {
                            prompt_tokens: u.prompt_tokens,
                            completion_tokens: u.completion_tokens,
                            total_tokens: u.total_tokens,
                        },
                    );
                }
                _ => {}
            }
        },
    )
    .await?;

    Ok(LlmStreamResult {
        content: sse_result.content,
        tool_calls: sse_result.tool_calls,
        usage: sse_result.usage,
        got_tool_calls_finish: sse_result.got_tool_calls_finish,
    })
}
