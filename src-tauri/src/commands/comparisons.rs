use std::collections::HashMap;
use std::sync::Arc;
use base64::Engine;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use sqlx::Row;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::commands::attachments::save_attachment_file;
use crate::commands::chat::{
    is_image_generation_model, mcp_tools_to_openai, parse_tool_call_name,
    StreamState, ToolCallBuffer, MAX_TOOL_ITERATIONS,
};
use crate::models::chat::{AttachmentInput, ContentBlock, Message, StreamResponse};
use crate::models::comparison::{DbComparison, DbComparisonMessage};
use crate::services::http_client::build_http_client;
use crate::services::mcp_manager::McpManager;
use crate::services::retry::{retry_http_request, RetryResult};

type Pool = sqlx::SqlitePool;

fn now_unix() -> Result<i64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_comparison(
    pool: State<'_, Pool>,
    left_provider_id: String,
    left_model: String,
    right_provider_id: String,
    right_model: String,
    left_system_prompt: Option<String>,
    right_system_prompt: Option<String>,
) -> Result<DbComparison, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_unix()?;

    sqlx::query(
        "INSERT INTO comparisons (id, title, left_provider_id, left_model, left_system_prompt, right_provider_id, right_model, right_system_prompt, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind("Новое сравнение")
    .bind(&left_provider_id)
    .bind(&left_model)
    .bind(&left_system_prompt)
    .bind(&right_provider_id)
    .bind(&right_model)
    .bind(&right_system_prompt)
    .bind(now)
    .bind(now)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[create_comparison] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(DbComparison {
        id,
        title: "Новое сравнение".to_string(),
        left_provider_id,
        left_model,
        left_system_prompt,
        right_provider_id,
        right_model,
        right_system_prompt,
        created_at: now,
        updated_at: now,
    })
}

#[tauri::command]
pub async fn get_all_comparisons(pool: State<'_, Pool>) -> Result<Vec<DbComparison>, String> {
    let rows = sqlx::query(
        "SELECT id, title, left_provider_id, left_model, left_system_prompt, right_provider_id, right_model, right_system_prompt, created_at, updated_at FROM comparisons ORDER BY updated_at DESC",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_all_comparisons] SQL error: {}", e);
        e.to_string()
    })?;

    let comparisons = rows
        .into_iter()
        .map(|row| DbComparison {
            id: row.get("id"),
            title: row.get("title"),
            left_provider_id: row.get("left_provider_id"),
            left_model: row.get("left_model"),
            left_system_prompt: row.get("left_system_prompt"),
            right_provider_id: row.get("right_provider_id"),
            right_model: row.get("right_model"),
            right_system_prompt: row.get("right_system_prompt"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .collect();

    Ok(comparisons)
}

#[tauri::command]
pub async fn get_comparison(
    pool: State<'_, Pool>,
    comparison_id: String,
) -> Result<DbComparison, String> {
    let row = sqlx::query(
        "SELECT id, title, left_provider_id, left_model, left_system_prompt, right_provider_id, right_model, right_system_prompt, created_at, updated_at FROM comparisons WHERE id = ?",
    )
    .bind(&comparison_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_comparison] SQL error: {}", e);
        e.to_string()
    })?
    .ok_or_else(|| format!("Comparison not found: {}", comparison_id))?;

    Ok(DbComparison {
        id: row.get("id"),
        title: row.get("title"),
        left_provider_id: row.get("left_provider_id"),
        left_model: row.get("left_model"),
        left_system_prompt: row.get("left_system_prompt"),
        right_provider_id: row.get("right_provider_id"),
        right_model: row.get("right_model"),
        right_system_prompt: row.get("right_system_prompt"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[tauri::command]
pub async fn delete_comparison(
    pool: State<'_, Pool>,
    comparison_id: String,
) -> Result<(), String> {
    sqlx::query("DELETE FROM comparison_messages WHERE comparison_id = ?")
        .bind(&comparison_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_comparison] SQL error (messages): {}", e);
            e.to_string()
        })?;

    sqlx::query("DELETE FROM comparisons WHERE id = ?")
        .bind(&comparison_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[delete_comparison] SQL error: {}", e);
            e.to_string()
        })?;

    Ok(())
}

#[tauri::command]
pub async fn update_comparison_title(
    pool: State<'_, Pool>,
    comparison_id: String,
    title: String,
) -> Result<(), String> {
    let now = now_unix()?;

    sqlx::query("UPDATE comparisons SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&title)
        .bind(now)
        .bind(&comparison_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_comparison_title] SQL error: {}", e);
            e.to_string()
        })?;

    Ok(())
}

#[tauri::command]
pub async fn get_comparison_messages(
    pool: State<'_, Pool>,
    comparison_id: String,
) -> Result<Vec<DbComparisonMessage>, String> {
    let rows = sqlx::query(
        "SELECT id, comparison_id, role, side, content, timestamp, model, prompt_tokens, completion_tokens, cost, has_attachments FROM comparison_messages WHERE comparison_id = ? ORDER BY timestamp ASC",
    )
    .bind(&comparison_id)
    .fetch_all(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[get_comparison_messages] SQL error: {}", e);
        e.to_string()
    })?;

    let messages = rows
        .into_iter()
        .map(|row| DbComparisonMessage {
            id: row.get("id"),
            comparison_id: row.get("comparison_id"),
            role: row.get("role"),
            side: row.get("side"),
            content: row.get("content"),
            timestamp: row.get("timestamp"),
            model: row.get("model"),
            prompt_tokens: row.try_get("prompt_tokens").unwrap_or(0),
            completion_tokens: row.try_get("completion_tokens").unwrap_or(0),
            cost: row.try_get("cost").unwrap_or(0.0),
            has_attachments: row.try_get("has_attachments").ok(),
        })
        .collect();

    Ok(messages)
}

#[tauri::command]
pub async fn save_comparison_message(
    pool: State<'_, Pool>,
    comparison_id: String,
    role: String,
    side: Option<String>,
    content: String,
    model: Option<String>,
) -> Result<DbComparisonMessage, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_unix()?;

    sqlx::query(
        "INSERT INTO comparison_messages (id, comparison_id, role, side, content, timestamp, model, prompt_tokens, completion_tokens, cost) VALUES (?, ?, ?, ?, ?, ?, ?, 0, 0, 0.0)",
    )
    .bind(&id)
    .bind(&comparison_id)
    .bind(&role)
    .bind(&side)
    .bind(&content)
    .bind(now)
    .bind(&model)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[save_comparison_message] SQL error: {}", e);
        e.to_string()
    })?;

    sqlx::query("UPDATE comparisons SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&comparison_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[save_comparison_message] SQL error (update parent): {}", e);
            e.to_string()
        })?;

    Ok(DbComparisonMessage {
        id,
        comparison_id,
        role,
        side,
        content,
        timestamp: now,
        model,
        prompt_tokens: 0,
        completion_tokens: 0,
        cost: 0.0,
        has_attachments: Some(0),
    })
}

#[tauri::command]
pub async fn update_comparison_message_usage(
    pool: State<'_, Pool>,
    message_id: String,
    prompt_tokens: i64,
    completion_tokens: i64,
    cost: f64,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE comparison_messages SET prompt_tokens = ?, completion_tokens = ?, cost = ? WHERE id = ?",
    )
    .bind(prompt_tokens)
    .bind(completion_tokens)
    .bind(cost)
    .bind(&message_id)
    .execute(pool.inner())
    .await
    .map_err(|e| {
        log::error!("[update_comparison_message_usage] SQL error: {}", e);
        e.to_string()
    })?;

    Ok(())
}

#[tauri::command]
pub async fn update_comparison_message_content(
    pool: State<'_, Pool>,
    message_id: String,
    content: String,
) -> Result<(), String> {
    sqlx::query("UPDATE comparison_messages SET content = ? WHERE id = ?")
        .bind(&content)
        .bind(&message_id)
        .execute(pool.inner())
        .await
        .map_err(|e| {
            log::error!("[update_comparison_message_content] SQL error: {}", e);
            e.to_string()
        })?;

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComparisonStreamPayload {
    side: String,
    content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComparisonStreamDonePayload {
    side: String,
    full_content: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComparisonStreamErrorPayload {
    side: String,
    error: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct ComparisonStreamUsagePayload {
    side: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComparisonStreamImagePayload {
    side: String,
    message_id: String,
    path: String,
    index: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComparisonStreamRetryPayload {
    side: String,
    attempt: u32,
    max_attempts: u32,
    delay_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComparisonToolCallPayload {
    side: String,
    tool_call_id: String,
    server_id: String,
    tool_name: String,
    arguments: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ComparisonToolResultPayload {
    side: String,
    tool_call_id: String,
    result: String,
    is_error: bool,
}

async fn stream_one_side(
    app: AppHandle,
    side: String,
    base_url: String,
    api_key: String,
    model: String,
    messages: Vec<serde_json::Value>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    assistant_message_id: String,
    pool: Pool,
    cancel_token: tokio_util::sync::CancellationToken,
    supports_tool_use: bool,
    openai_tools: Vec<serde_json::Value>,
    tool_name_map: HashMap<String, (String, String)>,
    mcp_manager: Arc<McpManager>,
) {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        if let Ok(hv) = format!("Bearer {}", api_key).parse() {
            headers.insert(AUTHORIZATION, hv);
        }
    }
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let is_image = is_image_generation_model(&model);
    let temperature = if is_image { None } else { temperature };
    let max_tokens = if is_image { None } else { max_tokens };

    let mut body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "stream_options": {"include_usage": true}
    });
    if let Some(t) = temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(m) = max_tokens {
        body["max_tokens"] = serde_json::json!(m);
    }
    if is_image {
        body["modalities"] = serde_json::json!(["image", "text"]);
    }

    if !is_image && supports_tool_use && !openai_tools.is_empty() {
        body["tools"] = serde_json::Value::Array(openai_tools.clone());
    }

    let client = match build_http_client(&app, None).await {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("comparison-stream-error", ComparisonStreamErrorPayload {
                side: side.clone(),
                error: e,
            });
            return;
        }
    };

    let mut full_content = String::new();
    let mut image_index: u32 = 0;
    let mut saved_images: Vec<(String, u32)> = Vec::new();
    let mut done_emitted = false;

    'tool_loop: for _iteration in 0..MAX_TOOL_ITERATIONS {
        let app_retry = app.clone();
        let side_retry = side.clone();
        let response = match retry_http_request(
            &client,
            &url,
            headers.clone(),
            &body,
            &cancel_token,
            |info| {
                let _ = app_retry.emit("comparison-stream-retry", ComparisonStreamRetryPayload {
                    side: side_retry.clone(),
                    attempt: info.attempt,
                    max_attempts: info.max_attempts,
                    delay_ms: info.delay_ms,
                });
            },
        ).await {
            RetryResult::Success(r) => r,
            RetryResult::Cancelled => break 'tool_loop,
            RetryResult::NonRetryableHttp { status, body: err_body } => {
                log::error!("[comparison-stream-{}] API error {}: {}", side, status, err_body);
                let parsed = serde_json::from_str::<serde_json::Value>(&err_body).ok();
                let msg = parsed
                    .as_ref()
                    .and_then(|v| v.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("API error ({})", status));
                let _ = app.emit("comparison-stream-error", ComparisonStreamErrorPayload {
                    side: side.clone(),
                    error: msg,
                });
                return;
            }
            RetryResult::Failed(e) => {
                let _ = app.emit("comparison-stream-error", ComparisonStreamErrorPayload {
                    side: side.clone(),
                    error: e,
                });
                return;
            }
        };

        let mut stream = response.bytes_stream();
        let mut sse_buffer = String::new();
        let mut tool_call_buffers: HashMap<usize, ToolCallBuffer> = HashMap::new();
        let mut got_tool_calls_finish = false;
        let mut iteration_content = String::new();
        let mut stream_done = false;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    break 'tool_loop;
                }
                chunk = stream.next() => {
                    match chunk {
                        None => { stream_done = true; break; }
                        Some(Err(e)) => {
                            let _ = app.emit("comparison-stream-error", ComparisonStreamErrorPayload {
                                side: side.clone(),
                                error: e.to_string(),
                            });
                            return;
                        }
                        Some(Ok(bytes)) => {
                            let text = String::from_utf8_lossy(&bytes);
                            sse_buffer.push_str(&text);

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
                                        if let Some(usage) = &resp.usage {
                                            let _ = app.emit("comparison-stream-usage", ComparisonStreamUsagePayload {
                                                side: side.clone(),
                                                prompt_tokens: usage.prompt_tokens,
                                                completion_tokens: usage.completion_tokens,
                                                total_tokens: usage.total_tokens,
                                            });
                                        }
                                        if let Some(choice) = resp.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                iteration_content.push_str(content);
                                                full_content.push_str(content);
                                                let _ = app.emit("comparison-stream", ComparisonStreamPayload {
                                                    side: side.clone(),
                                                    content: content.clone(),
                                                });
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

                                            if let Some(ref images) = choice.delta.images {
                                                for img in images.iter() {
                                                    let data_url = img.image_url.url.trim();
                                                    if let Some(base64_str) = data_url.splitn(2, ',').nth(1) {
                                                        match base64::engine::general_purpose::STANDARD.decode(base64_str.trim()) {
                                                            Ok(decoded) => {
                                                                let idx = image_index;
                                                                image_index += 1;
                                                                let file_name = format!("{}.png", idx);
                                                                if let Ok(rel_path) = save_attachment_file(
                                                                    &app,
                                                                    &assistant_message_id,
                                                                    &file_name,
                                                                    &decoded,
                                                                ) {
                                                                    saved_images.push((rel_path.clone(), idx));
                                                                    let _ = app.emit(
                                                                        "comparison-stream-image",
                                                                        ComparisonStreamImagePayload {
                                                                            side: side.clone(),
                                                                            message_id: assistant_message_id.clone(),
                                                                            path: rel_path,
                                                                            index: idx,
                                                                        },
                                                                    );
                                                                }
                                                            }
                                                            Err(e) => {
                                                                log::error!(
                                                                    "[comparison-stream-{}] image base64 decode error: {}",
                                                                    side,
                                                                    e
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if stream_done {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Tool call handling
        if (got_tool_calls_finish || !tool_call_buffers.is_empty())
            && !cancel_token.is_cancelled()
        {
            log::debug!(
                "[comparison-stream-{}] {} tool_call(s) in iteration {}",
                side, tool_call_buffers.len(), _iteration
            );

            let content_value = if iteration_content.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(iteration_content)
            };

            let mut sorted_indices: Vec<usize> =
                tool_call_buffers.keys().copied().collect();
            sorted_indices.sort();

            let tc_array: Vec<serde_json::Value> = sorted_indices
                .iter()
                .map(|idx| {
                    let buf = &tool_call_buffers[idx];
                    serde_json::json!({
                        "id": buf.id,
                        "type": "function",
                        "function": {
                            "name": buf.name,
                            "arguments": buf.arguments
                        }
                    })
                })
                .collect();

            if let Some(msgs) = body["messages"].as_array_mut() {
                msgs.push(serde_json::json!({
                    "role": "assistant",
                    "content": content_value,
                    "tool_calls": tc_array
                }));
            }

            for idx in &sorted_indices {
                if cancel_token.is_cancelled() {
                    break 'tool_loop;
                }

                let buf = &tool_call_buffers[idx];
                let (server_id, tool_name) = parse_tool_call_name(&buf.name, &tool_name_map);
                let args: serde_json::Value =
                    serde_json::from_str(&buf.arguments).unwrap_or(serde_json::json!({}));

                let _ = app.emit(
                    "comparison-stream-tool-call",
                    ComparisonToolCallPayload {
                        side: side.clone(),
                        tool_call_id: buf.id.clone(),
                        server_id: server_id.clone(),
                        tool_name: tool_name.clone(),
                        arguments: buf.arguments.clone(),
                    },
                );

                let result = mcp_manager.call_tool(&server_id, &tool_name, args).await;

                let (result_text, is_error) = match result {
                    Ok(r) => {
                        let text = r
                            .content
                            .iter()
                            .map(|c| c.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        (text, r.is_error)
                    }
                    Err(e) => (e.to_string(), true),
                };

                let _ = app.emit(
                    "comparison-stream-tool-result",
                    ComparisonToolResultPayload {
                        side: side.clone(),
                        tool_call_id: buf.id.clone(),
                        result: result_text.clone(),
                        is_error,
                    },
                );

                if let Some(msgs) = body["messages"].as_array_mut() {
                    msgs.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": buf.id,
                        "content": result_text
                    }));
                }
            }

            continue 'tool_loop;
        }

        // Normal completion
        done_emitted = true;
        break;
    }

    if !done_emitted && !cancel_token.is_cancelled() {
        log::warn!(
            "[comparison-stream-{}] tool loop exhausted {} iterations",
            side, MAX_TOOL_ITERATIONS
        );
    }

    let final_content = if saved_images.is_empty() {
        full_content.clone()
    } else {
        saved_images.sort_by_key(|(_, idx)| *idx);
        let mut blocks: Vec<ContentBlock> = Vec::new();
        blocks.push(ContentBlock {
            block_type: "text".to_string(),
            text: Some(full_content.clone()),
            image_url: None,
            path: None,
            name: None,
            mime: None,
        });
        for (path, idx) in &saved_images {
            blocks.push(ContentBlock {
                block_type: "image".to_string(),
                text: None,
                image_url: None,
                path: Some(path.clone()),
                name: Some(format!("image_{}.png", idx)),
                mime: None,
            });
        }
        serde_json::to_string(&blocks).unwrap_or_else(|_| full_content.clone())
    };

    let _ = sqlx::query("UPDATE comparison_messages SET content = ? WHERE id = ?")
        .bind(&final_content)
        .bind(&assistant_message_id)
        .execute(&pool)
        .await;

    let _ = app.emit("comparison-stream-done", ComparisonStreamDonePayload {
        side: side.clone(),
        full_content: final_content,
    });
}

#[tauri::command]
pub async fn stream_comparison_responses(
    app: AppHandle,
    stream_state: State<'_, StreamState>,
    pool: State<'_, Pool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    comparison_id: String,
    content: String,
    left_base_url: String,
    left_api_key: String,
    right_base_url: String,
    right_api_key: String,
    left_temperature: Option<f32>,
    left_max_tokens: Option<u32>,
    right_temperature: Option<f32>,
    right_max_tokens: Option<u32>,
    attachments: Option<Vec<AttachmentInput>>,
    left_supports_tool_use: Option<bool>,
    right_supports_tool_use: Option<bool>,
) -> Result<(), String> {
    let pool_inner = pool.inner().clone();
    let now = now_unix()?;

    let comparison = {
        let row = sqlx::query(
            "SELECT id, title, left_provider_id, left_model, left_system_prompt, right_provider_id, right_model, right_system_prompt, created_at, updated_at FROM comparisons WHERE id = ?",
        )
        .bind(&comparison_id)
        .fetch_optional(&pool_inner)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Comparison not found: {}", comparison_id))?;

        DbComparison {
            id: row.get("id"),
            title: row.get("title"),
            left_provider_id: row.get("left_provider_id"),
            left_model: row.get("left_model"),
            left_system_prompt: row.get("left_system_prompt"),
            right_provider_id: row.get("right_provider_id"),
            right_model: row.get("right_model"),
            right_system_prompt: row.get("right_system_prompt"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    };

    let user_msg_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO comparison_messages (id, comparison_id, role, side, content, timestamp, model, prompt_tokens, completion_tokens, cost) VALUES (?, ?, 'user', NULL, ?, ?, NULL, 0, 0, 0.0)",
    )
    .bind(&user_msg_id)
    .bind(&comparison_id)
    .bind(&content)
    .bind(now)
    .execute(&pool_inner)
    .await
    .map_err(|e| e.to_string())?;

    // Process attachments
    let mut api_blocks_for_current: Option<Vec<serde_json::Value>> = None;
    if let Some(ref atts) = attachments {
        if !atts.is_empty() {
            let mut db_blocks: Vec<ContentBlock> = Vec::new();
            let mut api_blocks: Vec<serde_json::Value> = Vec::new();

            for att in atts.iter() {
                let path = save_attachment_file(&app, &user_msg_id, &att.name, &att.data)?;
                let mime_lower = att.mime_type.to_lowercase();
                if mime_lower.starts_with("image/") {
                    db_blocks.push(ContentBlock {
                        block_type: "image".to_string(),
                        text: None,
                        image_url: None,
                        path: Some(path.clone()),
                        name: Some(att.name.clone()),
                        mime: None,
                    });
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&att.data);
                    api_blocks.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": format!("data:{};base64,{}", att.mime_type, b64) }
                    }));
                } else {
                    db_blocks.push(ContentBlock {
                        block_type: "file".to_string(),
                        text: None,
                        image_url: None,
                        path: Some(path.clone()),
                        name: Some(att.name.clone()),
                        mime: Some(att.mime_type.clone()),
                    });
                    let text_content = String::from_utf8_lossy(&att.data);
                    let file_block = format!("--- Файл: {} ---\n{}\n---", att.name, text_content);
                    api_blocks.push(serde_json::json!({ "type": "text", "text": file_block }));
                }
            }
            api_blocks.push(serde_json::json!({ "type": "text", "text": content }));
            db_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(content.clone()),
                image_url: None,
                path: None,
                name: None,
                mime: None,
            });

            let content_json = serde_json::to_string(&db_blocks).map_err(|e| e.to_string())?;
            sqlx::query("UPDATE comparison_messages SET content = ?, has_attachments = 1 WHERE id = ?")
                .bind(&content_json)
                .bind(&user_msg_id)
                .execute(&pool_inner)
                .await
                .map_err(|e| e.to_string())?;

            api_blocks_for_current = Some(api_blocks);
        }
    }

    let left_assistant_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO comparison_messages (id, comparison_id, role, side, content, timestamp, model, prompt_tokens, completion_tokens, cost) VALUES (?, ?, 'assistant', 'left', '', ?, ?, 0, 0, 0.0)",
    )
    .bind(&left_assistant_id)
    .bind(&comparison_id)
    .bind(now + 1)
    .bind(&comparison.left_model)
    .execute(&pool_inner)
    .await
    .map_err(|e| e.to_string())?;

    let right_assistant_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO comparison_messages (id, comparison_id, role, side, content, timestamp, model, prompt_tokens, completion_tokens, cost) VALUES (?, ?, 'assistant', 'right', '', ?, ?, 0, 0, 0.0)",
    )
    .bind(&right_assistant_id)
    .bind(&comparison_id)
    .bind(now + 1)
    .bind(&comparison.right_model)
    .execute(&pool_inner)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query("UPDATE comparisons SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(&comparison_id)
        .execute(&pool_inner)
        .await
        .map_err(|e| e.to_string())?;

    // Emit with content_json if attachments present, so frontend sees the blocks
    let emit_content = if api_blocks_for_current.is_some() {
        // Use the db_blocks JSON for the frontend to render attachments
        sqlx::query_scalar::<_, String>("SELECT content FROM comparison_messages WHERE id = ?")
            .bind(&user_msg_id)
            .fetch_one(&pool_inner)
            .await
            .unwrap_or_else(|_| content.clone())
    } else {
        content.clone()
    };

    let _ = app.emit("comparison-messages-saved", serde_json::json!({
        "userMessageId": user_msg_id,
        "leftAssistantId": left_assistant_id,
        "rightAssistantId": right_assistant_id,
        "content": emit_content,
        "timestamp": now,
        "comparisonId": comparison_id,
        "leftModel": comparison.left_model,
        "rightModel": comparison.right_model,
        "hasAttachments": api_blocks_for_current.is_some(),
    }));

    let existing_rows = sqlx::query(
        "SELECT role, side, content, has_attachments FROM comparison_messages WHERE comparison_id = ? AND id != ? AND id != ? AND id != ? ORDER BY timestamp ASC",
    )
    .bind(&comparison_id)
    .bind(&user_msg_id)
    .bind(&left_assistant_id)
    .bind(&right_assistant_id)
    .fetch_all(&pool_inner)
    .await
    .map_err(|e| e.to_string())?;

    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let build_history = |side_filter: &str, system_prompt: &Option<String>| -> Vec<serde_json::Value> {
        let mut msgs: Vec<serde_json::Value> = Vec::new();
        if let Some(sp) = system_prompt {
            if !sp.trim().is_empty() {
                msgs.push(serde_json::json!({ "role": "system", "content": sp }));
            }
        }
        for row in &existing_rows {
            let role: String = row.get("role");
            let side: Option<String> = row.get("side");
            let msg_content: String = row.get("content");
            let has_att: i64 = row.try_get("has_attachments").unwrap_or(0);
            if role == "user" {
                if has_att != 0 {
                    // Reconstruct API blocks from stored ContentBlock JSON
                    if let Ok(blocks) = serde_json::from_str::<Vec<ContentBlock>>(&msg_content) {
                        let mut api_parts: Vec<serde_json::Value> = Vec::new();
                        for block in &blocks {
                            match block.block_type.as_str() {
                                "image" => {
                                    if let Some(ref rel_path) = block.path {
                                        let full_path = app_data_dir.join(rel_path);
                                        if let Ok(data) = std::fs::read(&full_path) {
                                            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                                            let mime = block.mime.as_deref().unwrap_or("image/png");
                                            api_parts.push(serde_json::json!({
                                                "type": "image_url",
                                                "image_url": { "url": format!("data:{};base64,{}", mime, b64) }
                                            }));
                                        }
                                    }
                                }
                                "file" => {
                                    if let Some(ref rel_path) = block.path {
                                        let full_path = app_data_dir.join(rel_path);
                                        if let Ok(data) = std::fs::read(&full_path) {
                                            let text_content = String::from_utf8_lossy(&data);
                                            let name = block.name.as_deref().unwrap_or("file");
                                            api_parts.push(serde_json::json!({
                                                "type": "text",
                                                "text": format!("--- Файл: {} ---\n{}\n---", name, text_content)
                                            }));
                                        }
                                    }
                                }
                                "text" => {
                                    if let Some(ref text) = block.text {
                                        api_parts.push(serde_json::json!({ "type": "text", "text": text }));
                                    }
                                }
                                _ => {}
                            }
                        }
                        msgs.push(serde_json::json!({ "role": "user", "content": api_parts }));
                    } else {
                        msgs.push(serde_json::json!({ "role": "user", "content": msg_content }));
                    }
                } else {
                    msgs.push(serde_json::json!({ "role": "user", "content": msg_content }));
                }
            } else if role == "assistant" {
                if let Some(ref s) = side {
                    if s == side_filter {
                        msgs.push(serde_json::json!({ "role": "assistant", "content": msg_content }));
                    }
                }
            }
        }
        // Current user message
        if let Some(ref blocks) = api_blocks_for_current {
            msgs.push(serde_json::json!({ "role": "user", "content": blocks }));
        } else {
            msgs.push(serde_json::json!({ "role": "user", "content": content }));
        }
        msgs
    };

    let left_messages = build_history("left", &comparison.left_system_prompt);
    let right_messages = build_history("right", &comparison.right_system_prompt);

    // Prepare MCP tools
    let mcp_mgr = mcp_manager.inner().clone();
    let mcp_tools = mcp_mgr.get_all_tools().await;
    let (openai_tools, tool_name_map) = mcp_tools_to_openai(&mcp_tools);

    let cancel_token = tokio_util::sync::CancellationToken::new();
    *stream_state.comparison_cancel_token.lock().await = Some(cancel_token.clone());

    let app_left = app.clone();
    let app_right = app.clone();
    let pool_left = pool_inner.clone();
    let pool_right = pool_inner.clone();
    let cancel_left = cancel_token.clone();
    let cancel_right = cancel_token.clone();
    let left_model = comparison.left_model.clone();
    let right_model = comparison.right_model.clone();
    let left_aid = left_assistant_id.clone();
    let right_aid = right_assistant_id.clone();

    let left_tools = openai_tools.clone();
    let right_tools = openai_tools;
    let left_name_map = tool_name_map.clone();
    let right_name_map = tool_name_map;
    let mcp_left = mcp_mgr.clone();
    let mcp_right = mcp_mgr;

    let left_handle = tokio::spawn(stream_one_side(
        app_left,
        "left".to_string(),
        left_base_url,
        left_api_key,
        left_model,
        left_messages,
        left_temperature,
        left_max_tokens,
        left_aid,
        pool_left,
        cancel_left,
        left_supports_tool_use.unwrap_or(true),
        left_tools,
        left_name_map,
        mcp_left,
    ));

    let right_handle = tokio::spawn(stream_one_side(
        app_right,
        "right".to_string(),
        right_base_url,
        right_api_key,
        right_model,
        right_messages,
        right_temperature,
        right_max_tokens,
        right_aid,
        pool_right,
        cancel_right,
        right_supports_tool_use.unwrap_or(true),
        right_tools,
        right_name_map,
        mcp_right,
    ));

    let _ = tokio::join!(left_handle, right_handle);

    *stream_state.comparison_cancel_token.lock().await = None;

    if comparison.title == "Новое сравнение" || comparison.title == "New comparison" {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            let title = {
                let mut chars = trimmed.chars();
                let truncated: String = chars.by_ref().take(60).collect();
                let has_more = chars.next().is_some();
                if has_more {
                    match truncated.rfind(' ') {
                        Some(pos) if pos > 0 => format!("{}…", truncated[..pos].trim_end()),
                        _ => format!("{}…", truncated.trim_end()),
                    }
                } else {
                    truncated
                }
            };
            let _ = sqlx::query("UPDATE comparisons SET title = ?, updated_at = ? WHERE id = ?")
                .bind(&title)
                .bind(now)
                .bind(&comparison_id)
                .execute(&pool_inner)
                .await;
            let _ = app.emit("comparison-title-updated", serde_json::json!({
                "comparisonId": comparison_id,
                "title": title,
            }));
        }
    }

    Ok(())
}
