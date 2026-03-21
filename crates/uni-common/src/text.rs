/// Approximate token count. Conservative: ~3 chars per token (handles multilingual).
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 2) / 3
}

/// UTF-8 safe string truncation by byte limit.
/// Never splits a multi-byte character. Returns a borrowed slice.
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// UTF-8 safe truncation by character count, appending "..." if truncated.
/// Returns an owned String.
pub fn safe_truncate_chars(s: &str, max_chars: usize) -> String {
    let mut end = 0;
    let mut count = 0;
    for (i, c) in s.char_indices() {
        if count >= max_chars {
            let mut result = s[..i].to_string();
            result.push_str("...");
            return result;
        }
        end = i + c.len_utf8();
        count += 1;
    }
    s[..end].to_string()
}
