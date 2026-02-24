// команда отправки сообщения в OpenRouter (со стримингом)
use std::sync::Arc;
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::models::chat::{ChatRequest, Message, StreamResponse};

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
    api_key: String,
    model: String,
    messages: Vec<Message>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
) -> Result<(), String> {
    // Собираем запрос
    let request_body = ChatRequest {
        model,
        messages,
        stream: true,
        temperature,
        max_tokens,
    };

    // Заголовки
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, format!("Bearer {}", api_key).parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    // Отправляем запрос
    let client = reqwest::Client::new();
    let response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
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