use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;
use tauri::{AppHandle, Manager};

use uni_ssh::SshTunnelManager;

/// Get the active proxy URL from SSH tunnel (the only proxy source).
pub async fn get_active_proxy_url(app: &AppHandle) -> Option<String> {
    if let Some(manager) = app.try_state::<Arc<SshTunnelManager>>() {
        if let Some(url) = manager.get_proxy_url().await {
            return Some(url);
        }
    }
    None
}

/// Build reqwest::Client with proxy from app settings store.
pub async fn build_http_client(app: &AppHandle, timeout: Option<Duration>) -> Result<Client, String> {
    let proxy_url = get_active_proxy_url(app).await;
    uni_http::build_http_client(proxy_url.as_deref(), timeout)
}
