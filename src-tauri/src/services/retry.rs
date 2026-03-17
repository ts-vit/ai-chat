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

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    #[test]
    fn test_status_429_retryable() {
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
    }

    #[test]
    fn test_status_500_502_503_504_retryable() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::BAD_GATEWAY));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_retryable_status(StatusCode::GATEWAY_TIMEOUT));
    }

    #[test]
    fn test_status_401_not_retryable() {
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn test_status_400_not_retryable() {
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn test_status_200_not_retryable() {
        assert!(!is_retryable_status(StatusCode::OK));
    }

    #[test]
    fn test_compute_delay_exponential() {
        let d0 = compute_delay(0, None);
        assert_eq!(d0.as_millis(), 1000); // BASE_DELAY_MS * 2^0 = 1000

        let d1 = compute_delay(1, None);
        assert_eq!(d1.as_millis(), 2000); // 1000 * 2^1

        let d2 = compute_delay(2, None);
        assert_eq!(d2.as_millis(), 4000); // 1000 * 2^2
    }

    #[test]
    fn test_compute_delay_with_retry_after() {
        let d = compute_delay(0, Some(5));
        assert_eq!(d.as_millis(), 5000);
    }

    #[test]
    fn test_compute_delay_capped_at_max() {
        // attempt=20 → 1000 * 2^20 = 1_048_576_000 → capped at 30000
        let d = compute_delay(20, None);
        assert_eq!(d.as_millis(), MAX_DELAY_MS as u128);
    }

    #[test]
    fn test_compute_delay_retry_after_capped() {
        // retry_after = 60s → 60000ms, capped at 30000
        let d = compute_delay(0, Some(60));
        assert_eq!(d.as_millis(), MAX_DELAY_MS as u128);
    }

    #[test]
    fn test_parse_retry_after_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("5"));
        assert_eq!(parse_retry_after(&headers), Some(5));
    }

    #[test]
    fn test_parse_retry_after_missing() {
        let headers = HeaderMap::new();
        assert_eq!(parse_retry_after(&headers), None);
    }

    #[test]
    fn test_parse_retry_after_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("not-a-number"));
        assert_eq!(parse_retry_after(&headers), None);
    }
}
