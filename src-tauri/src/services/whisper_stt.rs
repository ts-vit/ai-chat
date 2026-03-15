use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(serde::Deserialize)]
struct WhisperResponse {
    text: String,
}

#[derive(serde::Deserialize)]
struct WhisperError {
    error: Option<WhisperErrorDetail>,
}

#[derive(serde::Deserialize)]
struct WhisperErrorDetail {
    message: Option<String>,
}

pub async fn transcribe_whisper(
    client: &reqwest::Client,
    wav_bytes: Vec<u8>,
    api_key: &str,
    language: Option<&str>,
    base_url: &str,
    model: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err("API key is not set".to_string());
    }

    let file_part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Failed to create multipart part: {}", e))?;

    let url = format!(
        "{}/audio/transcriptions",
        base_url.trim_end_matches('/')
    );

    let mut form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", model.to_string());

    if let Some(lang) = language {
        form = form.text("language", lang.to_string());
    }

    log::debug!("Sending audio to Whisper API, url={}, language={:?}", url, language);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Whisper API request timed out (30s)".to_string()
            } else {
                format!("Whisper API request failed: {}", e)
            }
        })?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let err_msg = match status.as_u16() {
            401 => "Invalid API key".to_string(),
            429 => "Rate limit exceeded, try again later".to_string(),
            413 => "Audio file too large (limit 25MB)".to_string(),
            _ => {
                if let Ok(err) = serde_json::from_str::<WhisperError>(&body) {
                    err.error
                        .and_then(|e| e.message)
                        .unwrap_or_else(|| format!("Whisper API error {}", status))
                } else {
                    format!("Whisper API error {}: {}", status, body)
                }
            }
        };
        log::error!("Whisper API error: {}", err_msg);
        return Err(err_msg);
    }

    let resp: WhisperResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Whisper response: {}", e))?;

    log::debug!("Whisper transcription: {} chars", resp.text.len());
    Ok(resp.text)
}
