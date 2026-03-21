use std::io::{BufReader, Cursor};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use rodio::DeviceSinkBuilder;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_store::StoreExt;

use crate::services::audio_recorder::AudioRecorder;
use crate::services::http_client::build_http_client;

const SAMPLE_RATE: u32 = 16000;
const STORE_NAME: &str = "settings.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
}

pub(crate) struct TtsPlayback {
    handle: thread::JoinHandle<()>,
    stop_flag: Arc<AtomicBool>,
}

pub struct TtsState(pub(crate) std::sync::Mutex<Option<TtsPlayback>>);

impl TtsState {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }
}

#[tauri::command]
pub async fn start_recording(recorder: State<'_, Arc<AudioRecorder>>) -> Result<(), String> {
    recorder.start()
}

#[tauri::command]
pub async fn stop_recording_and_transcribe(
    recorder: State<'_, Arc<AudioRecorder>>,
    app: AppHandle,
    provider: String,
    language: Option<String>,
) -> Result<String, String> {
    let samples = recorder.stop()?;

    if samples.is_empty() {
        return Err("No audio data recorded".to_string());
    }

    log::info!(
        "Recorded {} samples ({:.1}s), provider={}",
        samples.len(),
        samples.len() as f64 / SAMPLE_RATE as f64,
        provider
    );

    match provider.as_str() {
        "whisper" => transcribe_whisper_api_openai(&app, &samples, language.as_deref()).await,
        "groq" => transcribe_whisper_api_groq(&app, &samples, language.as_deref()).await,
        _ => Err(format!(
            "Unknown STT provider: {}. Supported: whisper, groq",
            provider
        )),
    }
}

#[tauri::command]
pub fn is_recording(recorder: State<'_, Arc<AudioRecorder>>) -> Result<bool, String> {
    Ok(recorder.is_recording())
}

#[tauri::command]
pub async fn tts_speak(
    tts_state: State<'_, TtsState>,
    app: AppHandle,
    text: String,
    provider: String,
    voice: Option<String>,
) -> Result<(), String> {
    {
        let mut guard = tts_state.0.lock().map_err(|e| e.to_string())?;
        if let Some(playback) = guard.take() {
            playback.stop_flag.store(true, Ordering::Relaxed);
            let _ = playback.handle.join();
        }
    }

    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(());
    }

    if provider != "openai" {
        return Err(format!(
            "Unknown TTS provider: {}. Only 'openai' is supported.",
            provider
        ));
    }

    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let api_key = store
        .get("openaiApiKey")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    if api_key.is_empty() {
        return Err("OpenAI API key is not set. Set it in Settings → Audio.".to_string());
    }
    let tts_voice = store
        .get("ttsVoice")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "alloy".to_string());
    let tts_model = store
        .get("ttsModel")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "tts-1".to_string());
    let voice_param = voice.unwrap_or(tts_voice);

    let client = build_http_client(&app, Some(std::time::Duration::from_secs(60))).await?;
    let bytes = uni_audio::speak_openai(&client, text, &api_key, &voice_param, &tts_model).await?;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = Arc::clone(&stop_flag);
    let stop_flag_check = Arc::clone(&stop_flag);
    let app_clone = app.clone();
    let handle = thread::spawn(move || {
        play_openai_mp3(bytes, stop_flag_clone);
        if !stop_flag_check.load(Ordering::Relaxed) {
            app_clone.emit("tts-playback-done", ()).unwrap_or_default();
        }
    });

    let mut guard = tts_state.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(TtsPlayback { handle, stop_flag });
    Ok(())
}

fn play_openai_mp3(bytes: Vec<u8>, stop_flag: Arc<AtomicBool>) {
    let reader = BufReader::new(Cursor::new(bytes));
    let handle = match DeviceSinkBuilder::open_default_sink() {
        Ok(h) => h,
        Err(e) => {
            log::error!("TTS open audio sink: {}", e);
            return;
        }
    };
    let player = match rodio::play(handle.mixer(), reader) {
        Ok(p) => p,
        Err(e) => {
            log::error!("TTS play: {}", e);
            return;
        }
    };
    while !stop_flag.load(Ordering::Relaxed) && !player.empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if stop_flag.load(Ordering::Relaxed) {
        player.stop();
    }
}

#[tauri::command]
pub async fn tts_stop(tts_state: State<'_, TtsState>) -> Result<(), String> {
    let mut guard = tts_state.0.lock().map_err(|e| e.to_string())?;
    if let Some(playback) = guard.take() {
        playback.stop_flag.store(true, Ordering::Relaxed);
        let _ = playback.handle.join();
    }
    Ok(())
}

#[tauri::command]
pub async fn tts_get_voices(provider: String) -> Result<Vec<TtsVoice>, String> {
    match provider.as_str() {
        "openai" => Ok(uni_audio::openai_voice_ids()
            .iter()
            .map(|id| TtsVoice {
                id: id.to_string(),
                name: id.to_string(),
                language: "en".to_string(),
            })
            .collect()),
        _ => Err(format!("Unknown TTS provider: {}", provider)),
    }
}

async fn transcribe_whisper_api_openai(
    app: &AppHandle,
    samples: &[i16],
    language: Option<&str>,
) -> Result<String, String> {
    let api_key = load_openai_api_key(app)?;
    let wav_bytes = encode_wav(samples)?;
    let client = build_http_client(app, Some(std::time::Duration::from_secs(30))).await?;
    uni_audio::transcribe_whisper(
        &client,
        wav_bytes,
        &api_key,
        language,
        "https://api.openai.com/v1",
        "whisper-1",
    )
    .await
}

async fn transcribe_whisper_api_groq(
    app: &AppHandle,
    samples: &[i16],
    language: Option<&str>,
) -> Result<String, String> {
    let api_key = load_groq_stt_api_key(app)?;
    let wav_bytes = encode_wav(samples)?;
    let client = build_http_client(app, Some(std::time::Duration::from_secs(30))).await?;
    uni_audio::transcribe_whisper(
        &client,
        wav_bytes,
        &api_key,
        language,
        "https://api.groq.com/openai/v1",
        "whisper-large-v3-turbo",
    )
    .await
}

fn load_openai_api_key(app: &AppHandle) -> Result<String, String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let key = store
        .get("openaiApiKey")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    if key.is_empty() {
        return Err("OpenAI API key is not configured. Set it in Settings → Audio.".to_string());
    }

    Ok(key)
}

fn load_groq_stt_api_key(app: &AppHandle) -> Result<String, String> {
    let store = app.store(STORE_NAME).map_err(|e| e.to_string())?;
    let key = store
        .get("groqSttApiKey")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    if key.is_empty() {
        return Err("Groq API key is not configured. Set it in Settings → Audio.".to_string());
    }

    Ok(key)
}

fn encode_wav(samples: &[i16]) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer =
            hound::WavWriter::new(&mut cursor, spec).map_err(|e| format!("WAV encode error: {}", e))?;
        for &sample in samples {
            writer
                .write_sample(sample)
                .map_err(|e| format!("WAV write error: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("WAV finalize error: {}", e))?;
    }

    Ok(cursor.into_inner())
}
