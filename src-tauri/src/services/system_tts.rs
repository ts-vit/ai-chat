//! System TTS via the `tts` crate (Windows SAPI/WinRT, macOS AVFoundation, Linux speech-dispatcher).

use serde::Serialize;
use tts::{Backends, Tts};

#[cfg(windows)]
fn default_backend() -> Backends {
    Backends::WinRt
}

#[cfg(target_os = "linux")]
fn default_backend() -> Backends {
    Backends::SpeechDispatcher
}

#[cfg(target_os = "macos")]
fn default_backend() -> Backends {
    Backends::AvFoundation
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn default_backend() -> Backends {
    #[allow(unreachable_code)]
    {
        Backends::SpeechDispatcher
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemVoice {
    pub id: String,
    pub name: String,
    pub language: String,
}

/// Returns the list of available system voices.
pub fn get_system_voices() -> Result<Vec<SystemVoice>, String> {
    let tts = Tts::new(default_backend()).map_err(|e| e.to_string())?;
    let voices = tts.voices().map_err(|e| e.to_string())?;
    Ok(voices
        .into_iter()
        .map(|v| SystemVoice {
            id: v.id(),
            name: v.name(),
            language: v.language().to_string(),
        })
        .collect())
}

/// Speaks the given text using the system TTS. If `voice` is Some(id), sets the voice by id.
/// If `rate` is Some, sets the speech rate (clamped to engine min/max).
/// The Tts instance must be kept alive for playback to continue; call stop() on it to stop.
pub fn speak_system(
    tts: &mut Tts,
    text: String,
    voice_id: Option<String>,
    rate: Option<f32>,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    if let Some(id) = voice_id {
        let voices = tts.voices().map_err(|e| e.to_string())?;
        if let Some(v) = voices.into_iter().find(|v| v.id() == id) {
            tts.set_voice(&v).map_err(|e| e.to_string())?;
        }
    }
    if let Some(r) = rate {
        let min = tts.min_rate();
        let max = tts.max_rate();
        let clamped = r.clamp(min, max);
        tts.set_rate(clamped).map_err(|e| e.to_string())?;
    }
    tts.speak(text, true).map_err(|e| e.to_string())?;
    Ok(())
}

/// Stops current system TTS playback.
pub fn stop_system(tts: &mut Tts) -> Result<(), String> {
    tts.stop().map_err(|e| e.to_string())?;
    Ok(())
}
