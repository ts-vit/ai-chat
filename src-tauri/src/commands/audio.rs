use std::io::{BufReader, Cursor};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use rodio::DeviceSinkBuilder;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;
use tts::Tts;

use crate::services::audio_recorder::AudioRecorder;
use crate::services::http_client::build_http_client;
use crate::services::openai_tts;
use crate::services::system_tts;
use crate::services::vosk_stt::{self, VoskStt};
use crate::services::whisper_stt;

const SAMPLE_RATE: u32 = 16000;
const STORE_NAME: &str = "settings.json";

const VOSK_RELEASE_VERSION: &str = "0.3.45";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
}

enum TtsPlayback {
    System(Tts),
    OpenAi(thread::JoinHandle<()>, Arc<AtomicBool>),
}

pub struct TtsState(pub(crate) std::sync::Mutex<Option<TtsPlayback>>);

impl TtsState {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoskModelInfo {
    pub language: String,
    pub path: String,
    pub size_mb: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoskStatus {
    pub library_installed: bool,
    pub models: Vec<VoskModelInfo>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
    percent: f64,
}

fn vosk_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("vosk"))
}

fn models_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(data.join("vosk-models"))
}

#[tauri::command]
pub async fn start_recording(
    recorder: State<'_, Arc<AudioRecorder>>,
) -> Result<(), String> {
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
        "vosk" => transcribe_vosk(&app, &samples, language.as_deref()),
        "whisper" => transcribe_whisper_api_openai(&app, &samples, language.as_deref()).await,
        "groq" => transcribe_whisper_api_groq(&app, &samples, language.as_deref()).await,
        _ => Err(format!("Unknown STT provider: {}", provider)),
    }
}

#[tauri::command]
pub async fn get_available_vosk_models(app: AppHandle) -> Result<Vec<VoskModelInfo>, String> {
    let dir = models_dir(&app)?;
    list_vosk_models(&dir)
}

#[tauri::command]
pub async fn check_vosk_status(app: AppHandle) -> Result<VoskStatus, String> {
    let vdir = vosk_dir(&app)?;
    let dll = vosk_stt::vosk_dll_path(&vdir);
    let library_installed = dll.exists();

    let mdir = models_dir(&app)?;
    let models = list_vosk_models(&mdir).unwrap_or_default();

    Ok(VoskStatus {
        library_installed,
        models,
    })
}

#[tauri::command]
pub async fn download_vosk_library(app: AppHandle) -> Result<(), String> {
    let vdir = vosk_dir(&app)?;
    std::fs::create_dir_all(&vdir).map_err(|e| format!("Failed to create vosk dir: {}", e))?;

    let dll_path = vosk_stt::vosk_dll_path(&vdir);
    if dll_path.exists() {
        if !vosk_stt::is_vosk_loaded() {
            vosk_stt::load_vosk_from_disk(&vdir)?;
        }
        return Ok(());
    }

    let (zip_url, dll_name_in_zip) = vosk_archive_url();

    log::info!("Downloading Vosk library from {}", zip_url);

    let client = build_http_client(&app, Some(std::time::Duration::from_secs(300))).await?;

    let resp = client
        .get(&zip_url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut bytes = Vec::with_capacity(total as usize);

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);

        let percent = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let _ = app.emit(
            "vosk-download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
            },
        );
    }

    log::info!("Downloaded {} bytes, extracting DLL...", bytes.len());

    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open zip: {}", e))?;

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("Zip entry error: {}", e))?;
        log::info!("Archive entry: {}", entry.name());
    }

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry error: {}", e))?;
        let raw_name = file.name().to_string();
        if file.is_dir() {
            continue;
        }
        let file_name = raw_name
            .rsplit(|c| c == '/' || c == '\\')
            .next()
            .unwrap_or(raw_name.as_str())
            .trim_end_matches('/')
            .trim_end_matches('\\');
        if file_name.is_empty() {
            continue;
        }
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut file, &mut buf)
            .map_err(|e| format!("Zip read error: {}", e))?;
        let target = vdir.join(file_name);
        std::fs::write(&target, &buf)
            .map_err(|e| format!("Failed to write {}: {}", file_name, e))?;
        log::info!("Extracted: {}", file_name);
    }

    if !vdir.join(&dll_name_in_zip).exists() {
        return Err(format!(
            "Could not find {} in downloaded archive",
            dll_name_in_zip
        ));
    }

    vosk_stt::load_vosk_from_disk(&vdir)?;
    log::info!("Vosk library installed and loaded from {:?}", dll_path);

    Ok(())
}

#[tauri::command]
pub async fn download_vosk_model(app: AppHandle, language: String) -> Result<(), String> {
    let mdir = models_dir(&app)?;
    let lang_dir = mdir.join(&language);

    if lang_dir.exists()
        && std::fs::read_dir(&lang_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false)
    {
        return Ok(());
    }

    let url = model_url(&language)?;
    log::info!("Downloading Vosk model for '{}' from {}", language, url);

    let client = build_http_client(&app, Some(std::time::Duration::from_secs(600))).await?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Model download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Model download failed: HTTP {}", resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut bytes = Vec::with_capacity(total as usize);

    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);

        let percent = if total > 0 {
            (downloaded as f64 / total as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let _ = app.emit(
            "vosk-model-download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
            },
        );
    }

    log::info!(
        "Downloaded {} bytes for model '{}', extracting...",
        bytes.len(),
        language
    );

    std::fs::create_dir_all(&lang_dir)
        .map_err(|e| format!("Failed to create model dir: {}", e))?;

    let cursor = Cursor::new(bytes);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|e| format!("Failed to open model zip: {}", e))?;

    // Vosk model zips contain a top-level folder like "vosk-model-small-en-us-0.15/".
    // We strip the first path component and extract contents directly into lang_dir.
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip entry error: {}", e))?;
        let raw_name = file.name().to_string();

        // Strip leading directory component
        let relative = match raw_name.find('/') {
            Some(pos) => &raw_name[pos + 1..],
            None => continue,
        };
        if relative.is_empty() {
            continue;
        }

        let target = lang_dir.join(relative);
        if file.is_dir() {
            let _ = std::fs::create_dir_all(&target);
        } else {
            if let Some(parent) = target.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut buf)
                .map_err(|e| format!("Zip read error: {}", e))?;
            std::fs::write(&target, &buf)
                .map_err(|e| format!("Failed to write model file: {}", e))?;
        }
    }

    log::info!("Vosk model '{}' installed to {:?}", language, lang_dir);
    Ok(())
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
            match playback {
                TtsPlayback::System(mut tts) => {
                    let _ = system_tts::stop_system(&mut tts);
                }
                TtsPlayback::OpenAi(handle, stop_flag) => {
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    let _ = handle.join();
                }
            }
        }
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(());
    }
    match provider.as_str() {
        "system" => {
            let mut tts = Tts::default().map_err(|e| e.to_string())?;
            system_tts::speak_system(&mut tts, text, voice, None)?;
            let mut guard = tts_state.0.lock().map_err(|e| e.to_string())?;
            *guard = Some(TtsPlayback::System(tts));
            app.emit("tts-playback-done", ()).unwrap_or_default();
            Ok(())
        }
        "openai" => {
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
            let bytes = openai_tts::speak_openai(&client, text, &api_key, &voice_param, &tts_model).await?;
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
            *guard = Some(TtsPlayback::OpenAi(handle, stop_flag));
            Ok(())
        }
        _ => Err(format!("Unknown TTS provider: {}", provider)),
    }
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
        match playback {
            TtsPlayback::System(mut tts) => {
                let _ = system_tts::stop_system(&mut tts);
            }
            TtsPlayback::OpenAi(handle, stop_flag) => {
                stop_flag.store(true, Ordering::Relaxed);
                let _ = handle.join();
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn tts_get_voices(provider: String) -> Result<Vec<TtsVoice>, String> {
    match provider.as_str() {
        "system" => {
            let voices = system_tts::get_system_voices()?;
            Ok(voices
                .into_iter()
                .map(|v| TtsVoice {
                    id: v.id,
                    name: v.name,
                    language: v.language,
                })
                .collect())
        }
        "openai" => Ok(openai_tts::openai_voice_ids()
            .iter()
            .map(|&id| TtsVoice {
                id: id.to_string(),
                name: id.to_string(),
                language: "en".to_string(),
            })
            .collect()),
        _ => Err(format!("Unknown TTS provider: {}", provider)),
    }
}

#[tauri::command]
pub async fn uninstall_vosk(app: AppHandle) -> Result<(), String> {
    vosk_stt::unload_vosk();
    let vdir = vosk_dir(&app)?;
    if vdir.exists() {
        std::fs::remove_dir_all(&vdir)
            .map_err(|e| format!("Failed to remove vosk directory: {}", e))?;
        log::info!("Vosk directory removed: {:?}", vdir);
    }
    Ok(())
}

fn transcribe_vosk(
    app: &AppHandle,
    samples: &[i16],
    language: Option<&str>,
) -> Result<String, String> {
    // Auto-load library if not yet loaded
    if !vosk_stt::is_vosk_loaded() {
        let vdir = vosk_dir(app)?;
        vosk_stt::load_vosk_from_disk(&vdir)?;
    }

    let lang = language.unwrap_or("en");
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let model_path = app_data.join("vosk-models").join(lang);

    if !model_path.exists() {
        return Err(format!(
            "Vosk model not found for language '{}'. Download it in Settings → Audio.",
            lang
        ));
    }

    let model_path_str = model_path.to_string_lossy().to_string();
    let stt = VoskStt::new(&model_path_str)?;
    stt.transcribe(samples, SAMPLE_RATE as f32)
}

async fn transcribe_whisper_api_openai(
    app: &AppHandle,
    samples: &[i16],
    language: Option<&str>,
) -> Result<String, String> {
    let api_key = load_openai_api_key(app)?;
    let wav_bytes = encode_wav(samples)?;
    let client = build_http_client(app, Some(std::time::Duration::from_secs(30))).await?;
    whisper_stt::transcribe_whisper(
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
    whisper_stt::transcribe_whisper(
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

fn list_vosk_models(models_dir: &std::path::Path) -> Result<Vec<VoskModelInfo>, String> {
    if !models_dir.exists() {
        return Ok(Vec::new());
    }

    let mut models = Vec::new();
    let entries = std::fs::read_dir(models_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let lang = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if lang.is_empty() {
            continue;
        }

        let size_mb = dir_size_mb(&path);
        if size_mb == 0 {
            continue;
        }

        models.push(VoskModelInfo {
            language: lang,
            path: path.to_string_lossy().to_string(),
            size_mb,
        });
    }

    Ok(models)
}

fn dir_size_mb(path: &std::path::Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += dir_size_mb(&p);
            }
        }
    }
    total / (1024 * 1024)
}

fn vosk_archive_url() -> (String, String) {
    #[cfg(target_os = "windows")]
    {
        (
            format!(
                "https://github.com/alphacep/vosk-api/releases/download/v{}/vosk-win64-{}.zip",
                VOSK_RELEASE_VERSION, VOSK_RELEASE_VERSION
            ),
            "libvosk.dll".to_string(),
        )
    }
    #[cfg(target_os = "linux")]
    {
        (
            format!(
                "https://github.com/alphacep/vosk-api/releases/download/v{}/vosk-linux-x86_64-{}.zip",
                VOSK_RELEASE_VERSION, VOSK_RELEASE_VERSION
            ),
            "libvosk.so".to_string(),
        )
    }
    #[cfg(target_os = "macos")]
    {
        (
            format!(
                "https://github.com/alphacep/vosk-api/releases/download/v{}/vosk-osx-{}.zip",
                VOSK_RELEASE_VERSION, VOSK_RELEASE_VERSION
            ),
            "libvosk.dylib".to_string(),
        )
    }
}

fn model_url(language: &str) -> Result<String, String> {
    let model_name = match language {
        "ru" => "vosk-model-ru-0.42",
        "en" => "vosk-model-small-en-us-0.15",
        "de" => "vosk-model-small-de-0.15",
        "fr" => "vosk-model-small-fr-0.22",
        "es" => "vosk-model-small-es-0.42",
        "zh" => "vosk-model-small-cn-0.22",
        "ja" => "vosk-model-small-ja-0.22",
        "ko" => "vosk-model-small-ko-0.22",
        _ => return Err(format!("Unsupported language for Vosk model: {}", language)),
    };
    Ok(format!(
        "https://alphacephei.com/vosk/models/{}.zip",
        model_name
    ))
}
