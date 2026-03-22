// App-level embedding provider from settings store (OpenAI / Gemini via uni-embedding)

use std::sync::Arc;
use tauri::{AppHandle, Manager};
use uni_settings::{JsonSettingsStore, SettingsStore};

use crate::services::http_client::build_http_client;

type SettingsState = Arc<JsonSettingsStore>;

async fn embedding_model_id_from_store(app: &AppHandle) -> Option<&'static str> {
    let store = app.state::<SettingsState>();
    let gemini_key = store
        .get("embedding.gemini.api_key")
        .await
        .unwrap_or_default()
        .filter(|s| !s.trim().is_empty());
    if gemini_key.is_some() {
        return Some("gemini");
    }
    let openai_key = store
        .get("embedding.openai.api_key")
        .await
        .unwrap_or_default()
        .filter(|s| !s.trim().is_empty());
    if openai_key.is_some() {
        return Some("openai");
    }
    None
}

/// Create an embedding provider from current app settings.
/// Returns None if no embedding API key is configured.
pub async fn get_embedding_provider(
    app: &AppHandle,
) -> Option<Box<dyn uni_embedding::EmbeddingProvider>> {
    let store = app.state::<SettingsState>();
    let gemini_key = store
        .get("embedding.gemini.api_key")
        .await
        .unwrap_or_default()
        .filter(|s| !s.is_empty());
    let openai_key = store
        .get("embedding.openai.api_key")
        .await
        .unwrap_or_default()
        .filter(|s| !s.is_empty());

    let (model_id, api_key) = if let Some(k) = gemini_key {
        ("gemini", k)
    } else if let Some(k) = openai_key {
        ("openai", k)
    } else {
        return None;
    };

    let client = build_http_client(app, None).await.ok()?;
    uni_embedding::create_embedding_provider(model_id, &api_key, client).ok()
}

/// LanceDB / vector index dimension for the configured embedding model.
/// When no keys are set, defaults to OpenAI small dimensions so tables stay stable until configured.
pub async fn get_embedding_dimensions(app: &AppHandle) -> usize {
    match embedding_model_id_from_store(app).await {
        Some("gemini") => uni_embedding::default_dimensions("gemini"),
        Some("openai") | None => uni_embedding::default_dimensions("openai"),
        _ => uni_embedding::default_dimensions("openai"),
    }
}
