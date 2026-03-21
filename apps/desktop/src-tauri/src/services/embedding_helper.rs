// App-level embedding provider from settings store (OpenAI / Gemini via uni-embedding)

use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::services::http_client::build_http_client;

const STORE_NAME: &str = "settings.json";

fn embedding_model_id_from_store(app: &AppHandle) -> Option<&'static str> {
    let store = app.store(STORE_NAME).ok()?;
    let gemini_key = store
        .get("embeddingGeminiKey")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.trim().is_empty());
    if gemini_key.is_some() {
        return Some("gemini");
    }
    let openai_key = store
        .get("embeddingOpenaiKey")
        .and_then(|v| v.as_str().map(String::from))
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
    let store = app.store(STORE_NAME).ok()?;
    let gemini_key = store
        .get("embeddingGeminiKey")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty());
    let openai_key = store
        .get("embeddingOpenaiKey")
        .and_then(|v| v.as_str().map(String::from))
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
pub fn get_embedding_dimensions(app: &AppHandle) -> usize {
    match embedding_model_id_from_store(app) {
        Some("gemini") => uni_embedding::default_dimensions("gemini"),
        Some("openai") | None => uni_embedding::default_dimensions("openai"),
        _ => uni_embedding::default_dimensions("openai"),
    }
}
