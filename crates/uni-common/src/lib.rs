pub mod error;
pub mod id;
pub mod text;
pub mod time;

pub use error::UniError;
pub use id::generate_id;
pub use text::{estimate_tokens, safe_truncate, safe_truncate_chars};
pub use time::now_unix_secs;
pub use tokio_util::sync::CancellationToken;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id = generate_id();
        assert_eq!(id.len(), 36); // UUID v4 format
        assert!(id.contains('-'));
    }

    #[test]
    fn test_now_unix_secs() {
        let ts = now_unix_secs();
        assert!(ts > 1_700_000_000); // after 2023
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abc"), 1);
        assert_eq!(estimate_tokens("hello world"), 4); // ceil(11/3)
    }

    #[test]
    fn test_safe_truncate() {
        assert_eq!(safe_truncate("hello", 10), "hello");
        assert_eq!(safe_truncate("hello", 3), "hel");
        // Russian text — UTF-8 multi-byte
        let russian = "Привет мир";
        let truncated = safe_truncate(russian, 6); // "При" = 6 bytes
        assert_eq!(truncated, "При");
        // Never splits a multi-byte char
        let truncated2 = safe_truncate(russian, 5); // would split "и", goes back to "Пр"
        assert_eq!(truncated2, "Пр");
    }

    #[test]
    fn test_safe_truncate_chars() {
        assert_eq!(safe_truncate_chars("hello world", 5), "hello...");
        assert_eq!(safe_truncate_chars("hi", 10), "hi");
        // Russian
        assert_eq!(safe_truncate_chars("Привет мир", 3), "При...");
    }
}
