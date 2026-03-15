// команда отправки сообщения в OpenRouter (со стримингом)
use std::collections::HashMap;
use std::sync::Arc;
use base64::Engine;
use sqlx::Row;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::commands::attachments::save_attachment_file;
use crate::commands::database::update_message_content_with_attachments;
use crate::models::chat::{AttachmentInput, ChatRequest, ContentBlock, Message, StreamResponse};
use crate::models::mcp::McpTool;
use crate::services::http_client::build_http_client;
use crate::services::mcp_manager::McpManager;
use crate::services::retry::{retry_http_request, RetryResult};

pub(crate) const MAX_TOOL_ITERATIONS: usize = 10;

pub(crate) fn mcp_tools_to_openai(
    tools: &[(String, McpTool)],
) -> (Vec<serde_json::Value>, HashMap<String, (String, String)>) {
    let mut openai_tools = Vec::with_capacity(tools.len());
    let mut name_map: HashMap<String, (String, String)> = HashMap::new();

    for (server_id, tool) in tools {
        let mut safe_name = format!(
            "mcp__{}__{}",
            server_id.replace('-', "_"),
            tool.name.replace('-', "_")
        );
        if safe_name.len() > 64 {
            safe_name.truncate(64);
        }

        name_map.insert(safe_name.clone(), (server_id.clone(), tool.name.clone()));

        openai_tools.push(serde_json::json!({
            "type": "function",
            "function": {
                "name": safe_name,
                "description": tool.description.clone().unwrap_or_default(),
                "parameters": tool.input_schema.clone()
            }
        }));
    }

    (openai_tools, name_map)
}

pub(crate) fn parse_tool_call_name(name: &str, name_map: &HashMap<String, (String, String)>) -> (String, String) {
    if let Some(pair) = name_map.get(name) {
        return pair.clone();
    }
    match name.splitn(2, "__").collect::<Vec<_>>().as_slice() {
        [server_id, tool_name] => (server_id.to_string(), tool_name.to_string()),
        _ => (String::new(), name.to_string()),
    }
}

/// Определяет по ID модели, что это модель генерации изображений (fallback при отсутствии флага в БД).
pub fn is_image_generation_model(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("dall-e")
        || m.contains("gpt-image")
        || m.contains("-image-")
        || m.ends_with("-image")
        || m.contains("/flux")
        || m.contains("stable-diffusion")
        || m.contains("stabilityai")
        || m.contains("imagen")
        || m.contains("playground")
}

pub(crate) fn format_api_error(status: reqwest::StatusCode, body: &str, model: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let error_message = parsed
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str());

    if let Some(msg) = error_message {
        let lower = msg.to_lowercase();

        if lower.contains("tool use") || lower.contains("tool_use") || lower.contains("function calling") {
            return format!(
                "Модель {} не поддерживает вызов инструментов (MCP). \
                 Попробуйте использовать другую модель, например Claude, GPT-4o или Gemini.",
                model
            );
        }
        if lower.contains("context length") || lower.contains("token limit") {
            return "Сообщение слишком длинное для этой модели. Попробуйте сократить историю или начать новый чат.".to_string();
        }
        if lower.contains("rate limit") || lower.contains("429") {
            return "Слишком много запросов. Подождите немного и попробуйте снова.".to_string();
        }
        if lower.contains("invalid api key") || lower.contains("unauthorized") || lower.contains("401") {
            return "Неверный API-ключ. Проверьте ключ в настройках.".to_string();
        }
        if lower.contains("insufficient") && (lower.contains("credits") || lower.contains("balance") || lower.contains("quota")) {
            return "Недостаточно средств на балансе. Пополните баланс OpenRouter.".to_string();
        }

        return format!("Ошибка API ({}): {}", status.as_u16(), msg);
    }

    format!("Ошибка API ({})", status.as_u16())
}

/// Models that do not support system/developer instruction (e.g. gemma-3n). Returns false for such models.
fn model_supports_system(model: &str) -> bool {
    const NO_SYSTEM_PATTERNS: &[&str] = &["gemma-3n", "gemma3n"];
    let m = model.to_lowercase();
    !NO_SYSTEM_PATTERNS.iter().any(|p| m.contains(p))
}

/// If the model does not support system role, flatten system message into the first user message.
fn maybe_flatten_system(mut messages: Vec<Message>, model: &str) -> Vec<Message> {
    if model_supports_system(model) {
        return messages;
    }
    if messages.is_empty() || messages[0].role != "system" {
        return messages;
    }
    let system_content = messages.remove(0).content;
    if let Some(first_user) = messages.iter_mut().find(|m| m.role == "user") {
        first_user.content = format!(
            "[System instruction]\n{}\n[/System instruction]\n\n{}",
            system_content, first_user.content
        );
    }
    log::info!("[send_message] System prompt flattened for model: {}", model);
    messages
}

/// Состояние для отмены текущего стрима.
pub struct StreamState {
    pub chat_cancel_token: Arc<Mutex<Option<CancellationToken>>>,
    pub comparison_cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

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
pub struct StreamImagePayload {
    pub message_id: String,
    pub path: String,
    pub index: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamUsagePayload {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
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
pub struct StreamRetryPayload {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_ms: u64,
}

pub(crate) struct ToolCallBuffer {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    stream_state: State<'_, StreamState>,
    pool: State<'_, sqlx::SqlitePool>,
    mcp_manager: State<'_, Arc<McpManager>>,
    chat_id: String,
    base_url: Option<String>,
    api_key: String,
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    user_message_id: Option<String>,
    attachments: Option<Vec<AttachmentInput>>,
    supports_image_generation: Option<bool>,
    supports_tool_use: Option<bool>,
    assistant_message_id: Option<String>,
    image_size: Option<String>,
    image_quality: Option<String>,
    image_style: Option<String>,
    image_n: Option<u32>,
    negative_prompt: Option<String>,
) -> Result<(), String> {
    if messages.is_empty() {
        return Err("Messages list is empty".to_string());
    }

    let (model, db_is_image_model, db_temperature, db_max_tokens, db_top_p, db_top_k, db_frequency_penalty, db_presence_penalty, db_image_size, db_image_quality, db_image_style, db_image_n, db_negative_prompt) = if !chat_id.is_empty() {
        let row = sqlx::query(
            "SELECT model, is_image_model, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty, image_size, image_quality, image_style, image_n, negative_prompt FROM chats WHERE id = ?",
        )
        .bind(&chat_id)
        .fetch_optional(pool.inner())
        .await
        .map_err(|e| e.to_string())?;
        match row {
            Some(r) => {
                let db_model = r.try_get::<String, _>("model").ok();
                let db_image = r.try_get::<i64, _>("is_image_model").unwrap_or(0) != 0;
                let resolved_model = db_model
                    .filter(|s: &String| !s.is_empty())
                    .unwrap_or_else(|| model.clone());
                let db_t = r.try_get::<f64, _>("temperature").ok().map(|v| v as f32);
                let db_m = r.try_get::<i64, _>("max_tokens").ok().map(|v| v as u32);
                let db_p = r.try_get::<f64, _>("top_p").ok().map(|v| v as f32);
                let db_k = r.try_get::<i64, _>("top_k").ok().map(|v| v as u32);
                let db_fp = r.try_get::<f64, _>("frequency_penalty").ok().map(|v| v as f32);
                let db_pp = r.try_get::<f64, _>("presence_penalty").ok().map(|v| v as f32);
                let db_is = r.try_get::<Option<String>, _>("image_size").ok().flatten();
                let db_iq = r.try_get::<Option<String>, _>("image_quality").ok().flatten();
                let db_ist = r.try_get::<Option<String>, _>("image_style").ok().flatten();
                let db_in = r.try_get::<Option<i64>, _>("image_n").ok().flatten().map(|v| v as u32);
                let db_np = r.try_get::<Option<String>, _>("negative_prompt").ok().flatten();
                (resolved_model, db_image, db_t, db_m, db_p, db_k, db_fp, db_pp, db_is, db_iq, db_ist, db_in, db_np)
            }
            None => (model.clone(), false, None, None, None, None, None, None, None, None, None, None, None),
        }
    } else {
        (model.clone(), false, None, None, None, None, None, None, None, None, None, None, None)
    };

    let messages = maybe_flatten_system(messages, &model);

    let is_image = db_is_image_model
        || is_image_generation_model(&model)
        || supports_image_generation == Some(true);

    let user_content = messages.last().map(|m| m.content.clone()).unwrap_or_default();
    let modalities = if is_image {
        Some(vec!["image".to_string(), "text".to_string()])
    } else {
        None
    };

    let no_text_params = is_image;
    let temperature = if no_text_params { None } else { db_temperature.or(temperature) };
    let max_tokens = if no_text_params { None } else { db_max_tokens.or(max_tokens) };
    let top_p = if no_text_params { None } else { db_top_p.or(top_p) };
    let top_k = if no_text_params { None } else { db_top_k.or(top_k) };
    let frequency_penalty = if no_text_params { None } else { db_frequency_penalty.or(frequency_penalty) };
    let presence_penalty = if no_text_params { None } else { db_presence_penalty.or(presence_penalty) };

    // Resolve style suffix from image_styles table
    let resolved_style_id = image_style.or(db_image_style);
    let style_suffix: Option<String> = if is_image {
        if let Some(ref style_id) = resolved_style_id {
            sqlx::query_scalar::<_, String>("SELECT prompt_suffix FROM image_styles WHERE id = ?")
                .bind(style_id)
                .fetch_optional(pool.inner())
                .await
                .map_err(|e| e.to_string())?
        } else {
            None
        }
    } else {
        None
    };

    // Resolve negative prompt
    let negative_prompt_resolved = negative_prompt.or(db_negative_prompt).filter(|s| !s.is_empty());

    let image_config = if is_image {
        let mut cfg = serde_json::Map::new();
        if let Some(s) = image_size.or(db_image_size) {
            cfg.insert("size".into(), serde_json::Value::String(s));
        }
        if let Some(q) = image_quality.or(db_image_quality) {
            cfg.insert("quality".into(), serde_json::Value::String(q));
        }
        // Note: style is NOT sent as API parameter anymore — it's used as prompt suffix
        if let Some(n) = image_n.or(db_image_n) {
            cfg.insert("n".into(), serde_json::json!(n));
        }
        if cfg.is_empty() { None } else { Some(serde_json::Value::Object(cfg)) }
    } else {
        None
    };

    // For image models: modify the last user message with style suffix (only for API, not DB)
    let mut messages = messages;
    if is_image {
        if let Some(last) = messages.last_mut() {
            if last.role == "user" {
                let mut final_prompt = last.content.clone();
                if let Some(ref suffix) = style_suffix {
                    final_prompt = format!("{}, {}", final_prompt, suffix);
                }
                if let Some(ref np) = negative_prompt_resolved {
                    final_prompt = format!("{}. Avoid: {}", final_prompt, np);
                }
                last.content = final_prompt;
            }
        }
    }

    log::debug!(
        "send_message: model={}, messages={}, temperature={:?}, max_tokens={:?}",
        model,
        messages.len(),
        temperature,
        max_tokens
    );

    let (request_body, _) = if let (Some(ref msg_id), Some(ref atts)) = (&user_message_id, &attachments) {
        if atts.is_empty() {
            let request_body = ChatRequest {
                model: model.clone(),
                messages: messages.clone(),
                stream: true,
                temperature,
                max_tokens,
                top_p,
                top_k,
                frequency_penalty,
                presence_penalty,
                stream_options: Some(serde_json::json!({"include_usage": true})),
                modalities: modalities.clone(),
                image_config: image_config.clone(),
            };
            (serde_json::to_value(&request_body).map_err(|e| e.to_string())?, None)
        } else {
            let mut db_blocks: Vec<ContentBlock> = Vec::new();
            let mut api_blocks: Vec<serde_json::Value> = Vec::new();

            for att in atts.iter() {
                let path = save_attachment_file(&app, msg_id, &att.name, &att.data)?;
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
            // For API: use modified content (with style suffix); for DB: use original
            let api_text = messages.last().map(|m| m.content.as_str()).unwrap_or(&user_content);
            api_blocks.push(serde_json::json!({ "type": "text", "text": api_text }));
            db_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(user_content.clone()),
                image_url: None,
                path: None,
                name: None,
                mime: None,
            });

            let content_json = serde_json::to_string(&db_blocks).map_err(|e| e.to_string())?;
            update_message_content_with_attachments(pool.inner(), msg_id, &content_json).await?;

            let mut messages_json: Vec<serde_json::Value> = messages
                .iter()
                .take(messages.len().saturating_sub(1))
                .map(|m| serde_json::json!({ "role": m.role, "content": m.content }))
                .collect();
            messages_json.push(serde_json::json!({
                "role": "user",
                "content": api_blocks
            }));

            let mut body = serde_json::json!({
                "model": model,
                "messages": messages_json,
                "stream": true,
                "stream_options": {"include_usage": true}
            });
            if temperature.is_some() {
                body["temperature"] = serde_json::to_value(temperature).unwrap();
            }
            if max_tokens.is_some() {
                body["max_tokens"] = serde_json::to_value(max_tokens).unwrap();
            }
            if let Some(ref mods) = modalities {
                body["modalities"] = serde_json::to_value(mods).unwrap();
            }
            if top_p.is_some() {
                body["top_p"] = serde_json::to_value(top_p).unwrap();
            }
            if top_k.is_some() {
                body["top_k"] = serde_json::to_value(top_k).unwrap();
            }
            if frequency_penalty.is_some() {
                body["frequency_penalty"] = serde_json::to_value(frequency_penalty).unwrap();
            }
            if presence_penalty.is_some() {
                body["presence_penalty"] = serde_json::to_value(presence_penalty).unwrap();
            }
            if let Some(ref ic) = image_config {
                if let Some(obj) = ic.as_object() {
                    for (k, v) in obj {
                        body[k] = v.clone();
                    }
                }
            }
            (body, Some(()))
        }
    } else {
        let request_body = ChatRequest {
            model: model.clone(),
            messages: messages.clone(),
            stream: true,
            temperature,
            max_tokens,
            top_p,
            top_k,
            frequency_penalty,
            presence_penalty,
            stream_options: Some(serde_json::json!({"include_usage": true})),
            modalities: modalities.clone(),
            image_config: image_config.clone(),
        };
        (serde_json::to_value(&request_body).map_err(|e| e.to_string())?, None)
    };

    let mut body = request_body;

    // Flatten image_config fields to top-level (OpenAI/OpenRouter expect size/quality/style/n at root)
    if let Some(ref ic) = image_config {
        if let Some(obj) = ic.as_object() {
            for (k, v) in obj {
                body[k] = v.clone();
            }
        }
        // Remove the nested image_config key
        if let Some(obj) = body.as_object_mut() {
            obj.remove("image_config");
        }
    }

    // Add negative_prompt to API request body if present
    if let Some(ref np) = negative_prompt_resolved {
        body["negative_prompt"] = serde_json::Value::String(np.clone());
    }

    // Inject MCP tools into request (skip for image models and models without tool support)
    let mcp_mgr = mcp_manager.inner().clone();
    let mut tool_name_map: HashMap<String, (String, String)> = HashMap::new();
    let tool_use_supported = supports_tool_use.unwrap_or(true);
    if !is_image && tool_use_supported {
        let mcp_tools = mcp_mgr.get_all_tools().await;
        if !mcp_tools.is_empty() {
            let (openai_tools, name_map) = mcp_tools_to_openai(&mcp_tools);
            body["tools"] = serde_json::Value::Array(openai_tools);
            tool_name_map = name_map;
            log::debug!("send_message: injected {} MCP tool(s)", mcp_tools.len());
        }
    }

    let base = base_url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));

    let mut headers = HeaderMap::new();
    if !api_key.is_empty() {
        if let Ok(hv) = format!("Bearer {}", api_key).parse() {
            headers.insert(AUTHORIZATION, hv);
        }
    }
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let client = build_http_client(&app, None).await?;

    let token = CancellationToken::new();
    *stream_state.chat_cancel_token.lock().await = Some(token.clone());

    let full_content = Arc::new(Mutex::new(String::new()));
    let full_content_cancel = full_content.clone();
    let app_stream = app.clone();
    let token_check = token.clone();
    let assistant_msg_id = assistant_message_id.clone();
    let image_index = Arc::new(Mutex::new(0u32));
    let chat_id_stream = chat_id.clone();

    let stream_result = tokio::select! {
        _ = token.cancelled() => {
            let fc = full_content_cancel.lock().await.clone();
            let _ = app.emit("chat-stream-done", StreamDonePayload {
                full_content: fc,
            });
            log::debug!("send_message: cancelled for chat_id={}", chat_id);
            *stream_state.chat_cancel_token.lock().await = None;
            return Ok(());
        }
        r = async move {
            let tool_name_map = tool_name_map;
            let mut done_emitted = false;

            'tool_loop: for _iteration in 0..MAX_TOOL_ITERATIONS {
                if _iteration > 0 {
                    log::debug!(
                        "send_message: tool loop iteration {} for chat_id={}",
                        _iteration,
                        chat_id_stream
                    );
                }

                let app_retry = app_stream.clone();
                let response = match retry_http_request(
                    &client,
                    &url,
                    headers.clone(),
                    &body,
                    &token_check,
                    |info| {
                        let _ = app_retry.emit("chat-stream-retry", StreamRetryPayload {
                            attempt: info.attempt,
                            max_attempts: info.max_attempts,
                            delay_ms: info.delay_ms,
                        });
                    },
                ).await {
                    RetryResult::Success(r) => r,
                    RetryResult::Cancelled => break 'tool_loop,
                    RetryResult::NonRetryableHttp { status, body: err_body } => {
                        let st = reqwest::StatusCode::from_u16(status)
                            .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
                        log::error!("send_message: API error {}: {}", st, err_body);
                        let user_error = format_api_error(st, &err_body, &model);
                        let _ = app_stream.emit(
                            "chat-stream-error",
                            StreamErrorPayload { error: user_error.clone() },
                        );
                        return Err(user_error);
                    }
                    RetryResult::Failed(e) => {
                        let _ = app_stream.emit(
                            "chat-stream-error",
                            StreamErrorPayload { error: e.clone() },
                        );
                        return Err(e);
                    }
                };

                let mut stream = response.bytes_stream();
                let mut sse_buffer = String::new();
                let mut tool_call_buffers: HashMap<usize, ToolCallBuffer> = HashMap::new();
                let mut got_tool_calls_finish = false;
                let mut iteration_content = String::new();
                let mut stream_done = false;

                while let Some(chunk) = stream.next().await {
                    if token_check.is_cancelled() {
                        break 'tool_loop;
                    }

                    let chunk = chunk.map_err(|e| e.to_string())?;
                    let text = String::from_utf8_lossy(&chunk);
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
                                    let _ = app_stream.emit(
                                        "chat-stream-usage",
                                        StreamUsagePayload {
                                            prompt_tokens: usage.prompt_tokens,
                                            completion_tokens: usage.completion_tokens,
                                            total_tokens: usage.total_tokens,
                                        },
                                    );
                                }
                                if let Some(choice) = resp.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        iteration_content.push_str(content);
                                        full_content.lock().await.push_str(content);
                                        let _ = app_stream.emit(
                                            "chat-stream",
                                            StreamPayload { content: content.clone() },
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

                                    if let Some(ref images) = choice.delta.images {
                                        if let Some(ref msg_id) = assistant_msg_id {
                                            for img in images.iter() {
                                                let data_url = img.image_url.url.trim();
                                                if let Some(base64_str) =
                                                    data_url.splitn(2, ',').nth(1)
                                                {
                                                    match base64::engine::general_purpose::STANDARD
                                                        .decode(base64_str.trim())
                                                    {
                                                        Ok(decoded) => {
                                                            let idx = {
                                                                let mut i =
                                                                    image_index.lock().await;
                                                                let n = *i;
                                                                *i += 1;
                                                                n
                                                            };
                                                            let file_name =
                                                                format!("{}.png", idx);
                                                            if let Ok(rel_path) =
                                                                save_attachment_file(
                                                                    &app_stream,
                                                                    msg_id,
                                                                    &file_name,
                                                                    &decoded,
                                                                )
                                                            {
                                                                let _ = app_stream.emit(
                                                                    "chat-stream-image",
                                                                    StreamImagePayload {
                                                                        message_id: msg_id
                                                                            .clone(),
                                                                        path: rel_path,
                                                                        index: idx,
                                                                    },
                                                                );
                                                            }
                                                        }
                                                        Err(e) => {
                                                            log::error!(
                                                                "[chat] image base64 decode error: {}",
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
                    }

                    if stream_done {
                        break;
                    }
                }

                // Tool call handling
                if (got_tool_calls_finish || !tool_call_buffers.is_empty())
                    && !token_check.is_cancelled()
                {
                    log::debug!(
                        "send_message: {} tool_call(s) in iteration {}",
                        tool_call_buffers.len(),
                        _iteration
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
                        if token_check.is_cancelled() {
                            break 'tool_loop;
                        }

                        let buf = &tool_call_buffers[idx];
                        let (server_id, tool_name) = parse_tool_call_name(&buf.name, &tool_name_map);
                        let args: serde_json::Value =
                            serde_json::from_str(&buf.arguments).unwrap_or(serde_json::json!({}));

                        log::debug!(
                            "send_message: calling tool {}/{} id={}",
                            server_id,
                            tool_name,
                            buf.id
                        );

                        let _ = app_stream.emit(
                            "chat-stream-tool-call",
                            StreamToolCallPayload {
                                tool_call_id: buf.id.clone(),
                                server_id: server_id.clone(),
                                tool_name: tool_name.clone(),
                                arguments: buf.arguments.clone(),
                            },
                        );

                        let result = mcp_mgr.call_tool(&server_id, &tool_name, args).await;

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

                        log::debug!(
                            "send_message: tool {} result: error={}, len={}",
                            buf.id,
                            is_error,
                            result_text.len()
                        );

                        let _ = app_stream.emit(
                            "chat-stream-tool-result",
                            StreamToolResultPayload {
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

                // Normal completion — no tool_calls
                let fc = full_content.lock().await.clone();
                let _ = app_stream.emit(
                    "chat-stream-done",
                    StreamDonePayload { full_content: fc },
                );
                log::debug!(
                    "send_message: stream completed for chat_id={}",
                    chat_id_stream
                );
                done_emitted = true;
                break;
            }

            if !done_emitted {
                let fc = full_content.lock().await.clone();
                let _ = app_stream.emit(
                    "chat-stream-done",
                    StreamDonePayload { full_content: fc },
                );
                if !token_check.is_cancelled() {
                    log::warn!(
                        "send_message: tool loop exhausted {} iterations for chat_id={}",
                        MAX_TOOL_ITERATIONS,
                        chat_id_stream
                    );
                }
            }

            Ok(())
        } => r
    };

    *stream_state.chat_cancel_token.lock().await = None;
    stream_result
}

#[tauri::command]
pub async fn stop_generation(stream_state: State<'_, StreamState>) -> Result<(), String> {
    if let Some(token) = stream_state.chat_cancel_token.lock().await.take() {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_comparison_generation(stream_state: State<'_, StreamState>) -> Result<(), String> {
    if let Some(token) = stream_state.comparison_cancel_token.lock().await.take() {
        token.cancel();
    }
    Ok(())
}
