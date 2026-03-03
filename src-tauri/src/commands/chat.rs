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

/// Определяет по ID модели, что это модель генерации изображений (fallback при отсутствии флага в БД).
fn is_image_generation_model(model: &str) -> bool {
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
    supports_image_generation: Option<bool>,
    assistant_message_id: Option<String>,
) -> Result<(), String> {
    // Модель, флаг image и per-чат параметры берём из чата в БД, fallback на переданные
    let (model, db_is_image_model, db_temperature, db_max_tokens, db_top_p, db_top_k, db_frequency_penalty, db_presence_penalty) = if !chat_id.is_empty() {
        let row = sqlx::query(
            "SELECT model, is_image_model, temperature, max_tokens, top_p, top_k, frequency_penalty, presence_penalty FROM chats WHERE id = ?",
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
                (resolved_model, db_image, db_t, db_m, db_p, db_k, db_fp, db_pp)
            }
            None => (model.clone(), false, None, None, None, None, None, None),
        }
    } else {
        (model.clone(), false, None, None, None, None, None, None)
    };

    let messages = maybe_flatten_system(messages, &model);

    let is_image = db_is_image_model
        || is_image_generation_model(&model)
        || supports_image_generation == Some(true);

    let user_content = messages.last().map(|m| m.content.as_str()).unwrap_or("");
    let modalities = if is_image {
        Some(vec!["image".to_string(), "text".to_string()])
    } else {
        None
    };

    // Для моделей генерации изображений не передаём параметры текстовой генерации (API возвращает 400).
    // Иначе: per-чат значение из БД приоритетнее, fallback — переданный с фронта (глобальные settings).
    let no_text_params = is_image;
    let temperature = if no_text_params { None } else { db_temperature.or(temperature) };
    let max_tokens = if no_text_params { None } else { db_max_tokens.or(max_tokens) };
    let top_p = if no_text_params { None } else { db_top_p.or(top_p) };
    let top_k = if no_text_params { None } else { db_top_k.or(top_k) };
    let frequency_penalty = if no_text_params { None } else { db_frequency_penalty.or(frequency_penalty) };
    let presence_penalty = if no_text_params { None } else { db_presence_penalty.or(presence_penalty) };

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
                image_config: None,
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
            image_config: None,
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
        log::error!("send_message: API error: {}", error);
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
    let assistant_msg_id = assistant_message_id.clone();
    let image_index = Arc::new(Mutex::new(0u32));
    let chat_id_stream = chat_id.clone();

    let stream_result = tokio::select! {
        _ = token.cancelled() => {
            let fc = full_content_emit.lock().await.clone();
            let _ = app.emit("chat-stream-done", StreamDonePayload {
                full_content: fc,
            });
            log::debug!("send_message: stream completed for chat_id={}", chat_id);
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
                            log::debug!("send_message: stream completed for chat_id={}", chat_id_stream);
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
                                if let Some(ref images) = choice.delta.images {
                                    if let Some(ref msg_id) = assistant_msg_id {
                                        for img in images.iter() {
                                            let data_url = img.image_url.url.trim();
                                            if let Some(base64_str) = data_url.splitn(2, ',').nth(1) {
                                                match base64::engine::general_purpose::STANDARD.decode(base64_str.trim()) {
                                                    Ok(decoded) => {
                                                        let idx = {
                                                            let mut i = image_index.lock().await;
                                                            let n = *i;
                                                            *i += 1;
                                                            n
                                                        };
                                                        let file_name = format!("{}.png", idx);
                                                        if let Ok(rel_path) = save_attachment_file(&app_stream, msg_id, &file_name, &decoded) {
                                                            let _ = app_stream.emit("chat-stream-image", StreamImagePayload {
                                                                message_id: msg_id.clone(),
                                                                path: rel_path,
                                                                index: idx,
                                                            });
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log::error!("[chat] image base64 decode error: {}", e);
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

                if token_check.is_cancelled() {
                    break;
                }
            }

            let fc = full_content.lock().await.clone();
            let _ = app_stream.emit("chat-stream-done", StreamDonePayload {
                full_content: fc,
            });
            log::debug!("send_message: stream completed for chat_id={}", chat_id_stream);
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