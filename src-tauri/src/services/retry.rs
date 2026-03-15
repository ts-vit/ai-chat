use reqwest::{Client, StatusCode, header::HeaderMap};
use serde::Serialize;
use tokio_util::sync::CancellationToken;
use std::time::Duration;

const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;
const BACKOFF_MULTIPLIER: f64 = 2.0;
const MAX_DELAY_MS: u64 = 30000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryInfo {
    pub attempt: u32,
    pub max_attempts: u32,
    pub delay_ms: u64,
}

pub enum RetryResult {
    Success(reqwest::Response),
    NonRetryableHttp { status: u16, body: String },
    Failed(String),
    Cancelled,
}

fn is_retryable_error(e: &reqwest::Error) -> bool {
    e.is_timeout() || e.is_connect() || e.is_request()
}

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 429 | 500 | 502 | 503 | 504)
}

fn compute_delay(attempt: u32, retry_after: Option<u64>) -> Duration {
    if let Some(secs) = retry_after {
        let ms = (secs * 1000).min(MAX_DELAY_MS);
        return Duration::from_millis(ms);
    }
    let ms = (BASE_DELAY_MS as f64 * BACKOFF_MULTIPLIER.powi(attempt as i32)) as u64;
    Duration::from_millis(ms.min(MAX_DELAY_MS))
}

fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

async fn wait_with_cancel(delay: Duration, cancel_token: &CancellationToken) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(delay) => false,
        _ = cancel_token.cancelled() => true,
    }
}

pub async fn retry_http_request(
    client: &Client,
    url: &str,
    headers: HeaderMap,
    body: &serde_json::Value,
    cancel_token: &CancellationToken,
    on_retry: impl Fn(RetryInfo),
) -> RetryResult {
    let total_attempts = MAX_RETRIES + 1;
    let mut last_error = String::new();
    let mut pending_retry_after: Option<u64> = None;

    for attempt in 0..total_attempts {
        if cancel_token.is_cancelled() {
            return RetryResult::Cancelled;
        }

        // Wait before retry (not on first attempt)
        if attempt > 0 {
            let delay = compute_delay(attempt - 1, pending_retry_after.take());
            on_retry(RetryInfo {
                attempt,
                max_attempts: total_attempts,
                delay_ms: delay.as_millis() as u64,
            });
            if wait_with_cancel(delay, cancel_token).await {
                return RetryResult::Cancelled;
            }
        }

        match client.post(url).headers(headers.clone()).json(body).send().await {
            Ok(response) => {
                let status = response.status();

                if status.is_success() {
                    return RetryResult::Success(response);
                }

                if is_retryable_status(status) && attempt < MAX_RETRIES {
                    pending_retry_after = parse_retry_after(response.headers());
                    let err_body = response.text().await.unwrap_or_default();
                    log::warn!(
                        "[retry] retryable HTTP {} on attempt {}/{}: {}",
                        status.as_u16(), attempt + 1, total_attempts, &err_body
                    );
                    last_error = format!("HTTP {}: {}", status.as_u16(), err_body);
                    continue;
                }

                // Non-retryable HTTP error (or last attempt for retryable)
                let err_body = response.text().await.unwrap_or_default();
                if is_retryable_status(status) {
                    // Last attempt for retryable status
                    return RetryResult::Failed(format!(
                        "Failed after {} attempts: HTTP {}: {}",
                        total_attempts, status.as_u16(), err_body
                    ));
                }
                return RetryResult::NonRetryableHttp {
                    status: status.as_u16(),
                    body: err_body,
                };
            }
            Err(e) => {
                if is_retryable_error(&e) && attempt < MAX_RETRIES {
                    log::warn!(
                        "[retry] retryable network error on attempt {}/{}: {}",
                        attempt + 1, total_attempts, e
                    );
                    last_error = e.to_string();
                    continue;
                }

                return RetryResult::Failed(if attempt > 0 {
                    format!("Failed after {} attempts: {}", attempt + 1, e)
                } else {
                    e.to_string()
                });
            }
        }
    }

    RetryResult::Failed(format!(
        "Failed after {} attempts: {}",
        total_attempts, last_error
    ))
}
