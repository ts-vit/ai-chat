use std::time::Duration;

use reqwest::Client;

/// Build reqwest::Client with optional proxy URL and timeout.
///
/// proxy_url: full proxy URL like "socks5://user:pass@host:port" or "http://host:port"
/// timeout: optional request timeout
pub fn build_http_client(
    proxy_url: Option<&str>,
    timeout: Option<Duration>,
) -> Result<Client, String> {
    let mut builder = Client::builder();

    if let Some(t) = timeout {
        builder = builder.timeout(t);
    }

    if let Some(url) = proxy_url {
        let proxy = reqwest::Proxy::all(url)
            .map_err(|e| format!("Invalid proxy URL: {}", e))?;
        builder = builder.proxy(proxy);
    }

    builder.build().map_err(|e| e.to_string())
}

/// Build reqwest::Client from explicit proxy parameters.
///
/// Constructs proxy URL from parts. Used for testing proxy connections.
pub fn build_http_client_from_params(
    proxy_type: &str,
    proxy_host: &str,
    proxy_port: u16,
    proxy_username: Option<&str>,
    proxy_password: Option<&str>,
    timeout: Option<Duration>,
) -> Result<Client, String> {
    if proxy_host.is_empty() {
        return build_http_client(None, timeout);
    }

    let proxy_url = match (proxy_username, proxy_password) {
        (Some(u), Some(p)) if !u.is_empty() && !p.is_empty() => {
            format!("{}://{}:{}@{}:{}", proxy_type, u, p, proxy_host, proxy_port)
        }
        _ => format!("{}://{}:{}", proxy_type, proxy_host, proxy_port),
    };

    build_http_client(Some(&proxy_url), timeout)
}
