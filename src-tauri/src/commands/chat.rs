// команда отправки сообщения в OpenRouter (со стримингом)
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

/// Состояние для отмены текущего стрима.
pub struct StreamState {
    pub cancel_token: Arc<Mutex<Option<CancellationToken>>>,
}

// Payload для событий стриминга — что получает фронтенд
#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamPayload {
    pub content: String,   // кусок текста
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamDonePayload {
    pub full_content: String, // полный ответ целиком
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamErrorPayload {
    pub error: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamUsagePayload {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    stream_state: State<'_, StreamState>,
    pool: State<'_, sqlx::SqlitePool>,
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
) -> Result<(), String> {
    // Модель берём из чата в БД (per-чат), fallback на переданный параметр
    let model = if !chat_id.is_empty() {
        let row = sqlx::query("SELECT model FROM chats WHERE id = ?")
            .bind(&chat_id)
            .fetch_optional(pool.inner())
            .await
            .map_err(|e| e.to_string())?;
        row.and_then(|r| r.try_get::<String, _>("model").ok())
            .filter(|s: &String| !s.is_empty())
            .unwrap_or(model)
    } else {
        model
    };

    let user_content = messages.last().map(|m| m.content.as_str()).unwrap_or("");

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
            api_blocks.push(serde_json::json!({ "type": "text", "text": user_content }));
            db_blocks.push(ContentBlock {
                block_type: "text".to_string(),
                text: Some(user_content.to_string()),
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
                "temperature": temperature,
                "max_tokens": max_tokens,
                "stream_options": {"include_usage": true}
            });
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
        };
        (serde_json::to_value(&request_body).map_err(|e| e.to_string())?, None)
    };

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

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .headers(headers)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            let _ = app.emit("chat-stream-error", StreamErrorPayload {
                error: e.to_string(),
            });
            e.to_string()
        })?;

    // Проверяем статус ответа
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let error = format!("API error {}: {}", status, body);
        let _ = app.emit("chat-stream-error", StreamErrorPayload {
            error: error.clone(),
        });
        return Err(error);
    }

    // Читаем стрим по частям
    let token = CancellationToken::new();
    *stream_state.cancel_token.lock().await = Some(token.clone());

    let mut stream = response.bytes_stream();
    let full_content = Arc::new(Mutex::new(String::new()));
    let full_content_emit = full_content.clone();
    let mut buffer = String::new();
    let app_stream = app.clone();
    let token_check = token.clone();

    let stream_result = tokio::select! {
        _ = token.cancelled() => {
            let fc = full_content_emit.lock().await.clone();
            let _ = app.emit("chat-stream-done", StreamDonePayload {
                full_content: fc,
            });
            *stream_state.cancel_token.lock().await = None;
            return Ok(());
        }
        r = async move {
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| e.to_string())?;
                let text = String::from_utf8_lossy(&chunk);

                buffer.push_str(&text);

                while let Some(line_end) = buffer.find('\n') {
                    let line = buffer[..line_end].trim().to_string();
                    buffer = buffer[line_end + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" {
                            let fc = full_content.lock().await.clone();
                            let _ = app_stream.emit("chat-stream-done", StreamDonePayload {
                                full_content: fc,
                            });
                            return Ok(());
                        }

                        if let Ok(response) = serde_json::from_str::<StreamResponse>(data) {
                            if let Some(usage) = &response.usage {
                                let _ = app_stream.emit(
                                    "chat-stream-usage",
                                    StreamUsagePayload {
                                        prompt_tokens: usage.prompt_tokens,
                                        completion_tokens: usage.completion_tokens,
                                        total_tokens: usage.total_tokens,
                                    },
                                );
                            }
                            if let Some(choice) = response.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    full_content.lock().await.push_str(content);

                                    let _ = app_stream.emit("chat-stream", StreamPayload {
                                        content: content.clone(),
                                    });
                                }
                            }
                        }
                    }
                }

                if token_check.is_cancelled() {
                    break;
                }
            }

            let fc = full_content.lock().await.clone();
            let _ = app_stream.emit("chat-stream-done", StreamDonePayload {
                full_content: fc,
            });
            Ok(())
        } => r
    };

    *stream_state.cancel_token.lock().await = None;
    stream_result
}

#[tauri::command]
pub async fn stop_generation(stream_state: State<'_, StreamState>) -> Result<(), String> {
    if let Some(token) = stream_state.cancel_token.lock().await.take() {
        token.cancel();
    }
    Ok(())
}