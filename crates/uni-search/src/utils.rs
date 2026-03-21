/// Escape a query string for FTS5 MATCH syntax.
/// Wraps multi-word queries in quotes, escapes internal quotes.
pub fn fts_escape_query(q: &str) -> String {
    let q = q.trim();
    if q.is_empty() {
        return "\"\"".to_string();
    }
    let escaped = q.replace('"', "\"\"");
    if !q.contains(char::is_whitespace) {
        escaped
    } else {
        format!("\"{}\"", escaped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_simple() {
        assert_eq!(fts_escape_query("hello"), "hello");
    }

    #[test]
    fn test_escape_multi_word() {
        assert_eq!(fts_escape_query("hello world"), "\"hello world\"");
    }

    #[test]
    fn test_escape_quotes() {
        assert_eq!(fts_escape_query("it's \"fine\""), "\"it's \"\"fine\"\"\"");
    }

    #[test]
    fn test_escape_empty() {
        assert_eq!(fts_escape_query(""), "\"\"");
        assert_eq!(fts_escape_query("  "), "\"\"");
    }
}
