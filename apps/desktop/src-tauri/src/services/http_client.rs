use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

use crate::services::ssh_tunnel::SshTunnelManager;

const STORE_NAME: &str = "settings.json";

/// Build proxy URL string from settings store. Returns None if proxy disabled or host empty.
pub fn build_proxy_url_from_store(app: &AppHandle) -> Option<String> {
    let store = app.store(STORE_NAME).ok()?;

    let proxy_enabled = store
        .get("proxyEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !proxy_enabled {
        return None;
    }

    let proxy_type = store
        .get("proxyType")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "http".to_string());
    let host = store
        .get("proxyHost")
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let port = store
        .get("proxyPort")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16)
        .unwrap_or(8080);
    let username = store
        .get("proxyUsername")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty());
    let password = store
        .get("proxyPassword")
        .and_then(|v| v.as_str().map(String::from))
        .filter(|s| !s.is_empty());

    if host.is_empty() {
        return None;
    }

    let url = match (username, password) {
        (Some(u), Some(p)) => format!("{}://{}:{}@{}:{}", proxy_type, u, p, host, port),
        _ => format!("{}://{}:{}", proxy_type, host, port),
    };
    Some(url)
}

/// Get the active proxy URL: SSH tunnel takes priority over manual proxy.
pub async fn get_active_proxy_url(app: &AppHandle) -> Option<String> {
    // 1. Check SSH tunnel first (highest priority)
    if let Some(manager) = app.try_state::<Arc<SshTunnelManager>>() {
        if let Some(url) = manager.get_proxy_url().await {
            return Some(url);
        }
    }
    // 2. Fall back to manual proxy settings
    build_proxy_url_from_store(app)
}

/// Build reqwest::Client with proxy from app settings store.
pub async fn build_http_client(app: &AppHandle, timeout: Option<Duration>) -> Result<Client, String> {
    let proxy_url = get_active_proxy_url(app).await;
    uni_http::build_http_client(proxy_url.as_deref(), timeout)
}
