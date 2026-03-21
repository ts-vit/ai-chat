pub mod stt;
pub mod tts;

// STT re-exports
pub use stt::transcribe_whisper;

// TTS re-exports
pub use tts::{speak_openai, openai_voice_ids, openai_model_ids};
