//! OpenAI TTS API: https://api.openai.com/v1/audio/speech

use std::time::Duration;

const OPENAI_TTS_URL: &str = "https://api.openai.com/v1/audio/speech";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

const OPENAI_VOICES: [&str; 6] = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
const OPENAI_MODELS: [&str; 2] = ["tts-1", "tts-1-hd"];

/// Returns the list of OpenAI voice ids.
pub fn openai_voice_ids() -> &'static [&'static str] {
    &OPENAI_VOICES
}

/// Returns the list of OpenAI model ids.
pub fn openai_model_ids() -> &'static [&'static str] {
    &OPENAI_MODELS
}

/// Calls OpenAI TTS API and returns mp3 bytes.
pub async fn speak_openai(
    text: String,
    api_key: &str,
    voice: &str,
    model: &str,
) -> Result<Vec<u8>, String> {
    if api_key.is_empty() {
        return Err("OpenAI API key is not set".to_string());
    }
    if text.trim().is_empty() {
        return Err("Text is empty".to_string());
    }
    let voice = if OPENAI_VOICES.contains(&voice) {
        voice
    } else {
        "alloy"
    };
    let model = if OPENAI_MODELS.contains(&model) {
        model
    } else {
        "tts-1"
    };

    let body = serde_json::json!({
        "model": model,
        "input": text,
        "voice": voice,
    });

    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    let response = client
        .post(OPENAI_TTS_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "OpenAI TTS request timed out".to_string()
            } else {
                format!("OpenAI TTS request failed: {}", e)
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let err_msg = match status.as_u16() {
            401 => "Invalid OpenAI API key".to_string(),
            429 => "OpenAI rate limit exceeded".to_string(),
            _ => {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&body) {
                    obj.get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or(&body)
                        .to_string()
                } else {
                    format!("OpenAI TTS error {}: {}", status, body)
                }
            }
        };
        return Err(err_msg);
    }

    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}
