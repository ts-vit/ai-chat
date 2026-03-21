use std::collections::HashMap;

use futures_util::StreamExt;
use uni_common::CancellationToken;

use crate::types::{StreamResponse, Usage};

/// Accumulated tool call from SSE deltas
#[derive(Debug, Clone)]
pub struct ToolCallBuffer {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Events emitted by the SSE parser
#[derive(Debug, Clone)]
pub enum SseEvent {
    /// Text content delta
    TextDelta(String),
    /// Tool call delta (accumulated per index)
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    /// Usage information
    Usage(Usage),
    /// Image delta (for image generation models)
    ImageDelta { url: String },
    /// Stream finished with tool_calls finish reason
    ToolCallsFinish,
    /// Stream finished normally
    Done,
}

/// Result of parsing a complete SSE stream
#[derive(Debug)]
pub struct SseStreamResult {
    pub content: String,
    pub tool_calls: Vec<ToolCallBuffer>,
    pub usage: Option<Usage>,
    pub got_tool_calls_finish: bool,
}

/// SSE parser for OpenAI-compatible streaming responses.
/// Pure function — no side effects, no Tauri, no events.
pub struct SseParser;

impl SseParser {
    /// Parse an SSE byte stream into a collected result.
    ///
    /// For each SSE event, calls `on_event` callback so the caller can
    /// emit Tauri events, log, or do whatever it needs.
    ///
    /// Returns the accumulated result (content + tool calls + usage).
    pub async fn parse_stream<S, F>(
        stream: S,
        cancel_token: &CancellationToken,
        mut on_event: F,
    ) -> Result<SseStreamResult, String>
    where
        S: futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin,
        F: FnMut(&SseEvent),
    {
        let mut byte_stream = stream;
        let mut sse_buffer = String::new();
        let mut tool_call_buffers: HashMap<usize, ToolCallBuffer> = HashMap::new();
        let mut got_tool_calls_finish = false;
        let mut content = String::new();
        let mut usage: Option<Usage> = None;

        while let Some(chunk) = byte_stream.next().await {
            if cancel_token.is_cancelled() {
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
                        on_event(&SseEvent::Done);
                        stream_done = true;
                        break;
                    }

                    if let Ok(resp) = serde_json::from_str::<StreamResponse>(data) {
                        // Usage
                        if let Some(u) = &resp.usage {
                            let evt = SseEvent::Usage(u.clone());
                            on_event(&evt);
                            usage = Some(u.clone());
                        }

                        if let Some(choice) = resp.choices.first() {
                            // Text content
                            if let Some(c) = &choice.delta.content {
                                content.push_str(c);
                                on_event(&SseEvent::TextDelta(c.clone()));
                            }

                            // Tool calls
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

                                    on_event(&SseEvent::ToolCallDelta {
                                        index: tc.index,
                                        id: tc.id.clone(),
                                        name: tc.function.as_ref().and_then(|f| f.name.clone()),
                                        arguments: tc.function.as_ref().and_then(|f| f.arguments.clone()),
                                    });
                                }
                            }

                            // Image deltas
                            if let Some(ref images) = choice.delta.images {
                                for img in images {
                                    on_event(&SseEvent::ImageDelta {
                                        url: img.image_url.url.clone(),
                                    });
                                }
                            }

                            // Finish reason
                            if choice.finish_reason.as_deref() == Some("tool_calls") {
                                got_tool_calls_finish = true;
                                on_event(&SseEvent::ToolCallsFinish);
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

        Ok(SseStreamResult {
            content,
            tool_calls: sorted_tool_calls,
            usage,
            got_tool_calls_finish,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a stream from SSE text
    fn mock_sse_stream(
        data: &str,
    ) -> impl futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Unpin {
        let chunks: Vec<Result<bytes::Bytes, reqwest::Error>> = data
            .as_bytes()
            .chunks(64)
            .map(|c| Ok(bytes::Bytes::copy_from_slice(c)))
            .collect();
        futures_util::stream::iter(chunks)
    }

    #[tokio::test]
    async fn test_parse_text_stream() {
        let sse_data = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" world\"},\"finish_reason\":null}]}\n\ndata: [DONE]\n\n";
        let stream = mock_sse_stream(sse_data);
        let cancel = CancellationToken::new();
        let mut events = Vec::new();

        let result = SseParser::parse_stream(stream, &cancel, |evt| {
            events.push(evt.clone());
        })
        .await
        .unwrap();

        assert_eq!(result.content, "Hello world");
        assert!(result.tool_calls.is_empty());
        // Check events contain TextDelta
        assert!(events.iter().any(|e| matches!(e, SseEvent::TextDelta(t) if t == "Hello")));
    }

    #[tokio::test]
    async fn test_parse_usage() {
        let sse_data = "data: {\"choices\":[{\"delta\":{},\"finish_reason\":null}],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20,\"total_tokens\":30}}\n\ndata: [DONE]\n\n";
        let stream = mock_sse_stream(sse_data);
        let cancel = CancellationToken::new();

        let result = SseParser::parse_stream(stream, &cancel, |_| {}).await.unwrap();
        let u = result.usage.unwrap();
        assert_eq!(u.prompt_tokens, 10);
        assert_eq!(u.total_tokens, 30);
    }

    #[tokio::test]
    async fn test_cancel() {
        let sse_data = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"},\"finish_reason\":null}]}\n\n";
        let stream = mock_sse_stream(sse_data);
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel immediately

        let result = SseParser::parse_stream(stream, &cancel, |_| {}).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cancelled"));
    }
}
