/// Re-render a heading with given level and text.
pub fn render_heading(level: u8, text: &str) -> String {
    let hashes = "#".repeat(level.min(6) as usize);
    format!("{} {}", hashes, text)
}

/// Render a code block with optional language.
pub fn render_code_block(language: &str, code: &str) -> String {
    if language.is_empty() {
        format!("```\n{}\n```", code)
    } else {
        format!("```{}\n{}\n```", language, code)
    }
}

/// Render a markdown link.
pub fn render_link(text: &str, url: &str, title: Option<&str>) -> String {
    if let Some(t) = title {
        format!("[{}]({} \"{}\")", text, url, t)
    } else {
        format!("[{}]({})", text, url)
    }
}

/// Render a bullet list from items.
pub fn render_list(items: &[&str]) -> String {
    items
        .iter()
        .map(|item| format!("- {}", item))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render a markdown table from headers and rows.
pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let mut result = String::new();

    // Header row
    result.push_str("| ");
    result.push_str(&headers.join(" | "));
    result.push_str(" |\n");

    // Separator
    result.push_str("| ");
    result.push_str(
        &headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | "),
    );
    result.push_str(" |\n");

    // Data rows
    for row in rows {
        result.push_str("| ");
        result.push_str(&row.join(" | "));
        result.push_str(" |\n");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_heading() {
        assert_eq!(render_heading(1, "Title"), "# Title");
        assert_eq!(render_heading(3, "Sub"), "### Sub");
    }

    #[test]
    fn test_render_code_block() {
        let result = render_code_block("rust", "fn main() {}");
        assert!(result.starts_with("```rust\n"));
        assert!(result.ends_with("\n```"));
    }

    #[test]
    fn test_render_code_block_no_lang() {
        let result = render_code_block("", "code");
        assert!(result.starts_with("```\n"));
    }

    #[test]
    fn test_render_link() {
        assert_eq!(
            render_link("Example", "https://example.com", None),
            "[Example](https://example.com)"
        );
        assert_eq!(
            render_link("Docs", "https://docs.rs", Some("Documentation")),
            "[Docs](https://docs.rs \"Documentation\")"
        );
    }

    #[test]
    fn test_render_list() {
        let result = render_list(&["Item 1", "Item 2", "Item 3"]);
        assert_eq!(result, "- Item 1\n- Item 2\n- Item 3");
    }

    #[test]
    fn test_render_table() {
        let result = render_table(
            &["Name", "Value"],
            &[
                vec!["A".to_string(), "1".to_string()],
                vec!["B".to_string(), "2".to_string()],
            ],
        );
        assert!(result.contains("| Name | Value |"));
        assert!(result.contains("| --- | --- |"));
        assert!(result.contains("| A | 1 |"));
    }

    #[test]
    fn test_render_table_empty() {
        let result = render_table(&[], &[]);
        assert!(result.is_empty());
    }
}
