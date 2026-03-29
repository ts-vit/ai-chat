use pulldown_cmark::{Event, Parser, Tag, TagEnd};

use crate::parse::{extract_headings, parser_options};

/// Table of Contents entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TocEntry {
    pub level: u8,
    pub text: String,
    /// Indentation based on level
    pub indent: usize,
}

/// Generate a Table of Contents from headings.
pub fn table_of_contents(markdown: &str) -> Vec<TocEntry> {
    extract_headings(markdown)
        .into_iter()
        .map(|h| TocEntry {
            indent: (h.level - 1) as usize,
            level: h.level,
            text: h.text,
        })
        .collect()
}

/// Render TOC as markdown list.
pub fn table_of_contents_md(markdown: &str) -> String {
    let toc = table_of_contents(markdown);
    toc.iter()
        .map(|entry| {
            let indent = "  ".repeat(entry.indent);
            format!("{}- {}", indent, entry.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract plain text from markdown (strip all formatting).
pub fn to_plain_text(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut text = String::new();

    for event in parser {
        match event {
            Event::Text(t) | Event::Code(t) => {
                text.push_str(&t);
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
            }
            Event::Start(Tag::Paragraph) => {
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            Event::End(TagEnd::CodeBlock) => {
                text.push('\n');
            }
            Event::Start(Tag::Item) => {
                text.push_str("- ");
            }
            _ => {}
        }
    }

    // Clean up: trim and collapse multiple newlines to max 2
    let trimmed = text.trim();
    let mut prev_newline = false;
    let mut cleaned = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch == '\n' {
            if prev_newline {
                continue;
            }
            prev_newline = true;
        } else {
            prev_newline = false;
        }
        cleaned.push(ch);
    }

    cleaned
}

/// Count words in markdown (plain text extraction first).
pub fn word_count(markdown: &str) -> usize {
    to_plain_text(markdown).split_whitespace().count()
}

/// Estimate reading time in minutes (assuming ~200 WPM).
pub fn reading_time_minutes(markdown: &str) -> u32 {
    let words = word_count(markdown);
    ((words as f64) / 200.0).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_of_contents() {
        let md = "# Title\n\n## Section 1\n\n### Sub A\n\n## Section 2";
        let toc = table_of_contents(md);
        assert_eq!(toc.len(), 4);
        assert_eq!(toc[0].level, 1);
        assert_eq!(toc[0].indent, 0);
        assert_eq!(toc[1].level, 2);
        assert_eq!(toc[1].indent, 1);
    }

    #[test]
    fn test_table_of_contents_md() {
        let md = "# Title\n\n## Section\n\n### Sub";
        let toc_md = table_of_contents_md(md);
        assert!(toc_md.contains("- Title"));
        assert!(toc_md.contains("  - Section"));
        assert!(toc_md.contains("    - Sub"));
    }

    #[test]
    fn test_to_plain_text() {
        let md = "# Title\n\n**Bold** and *italic* text\n\n- Item 1\n- Item 2";
        let plain = to_plain_text(md);
        assert!(plain.contains("Title"));
        assert!(plain.contains("Bold"));
        assert!(plain.contains("italic"));
        assert!(!plain.contains("**"));
        assert!(!plain.contains("#"));
    }

    #[test]
    fn test_to_plain_text_code_block() {
        let md = "Text\n\n```rust\nfn main() {}\n```\n\nMore text";
        let plain = to_plain_text(md);
        assert!(plain.contains("fn main()"));
        assert!(plain.contains("More text"));
        assert!(!plain.contains("```"));
    }

    #[test]
    fn test_word_count() {
        let md = "# Title\n\nThis is a **test** with some *words*.";
        let count = word_count(md);
        // "Title" + "This is a test with some words." = 8 words
        assert!(count >= 7 && count <= 10);
    }

    #[test]
    fn test_reading_time() {
        let words = "word ".repeat(200);
        let md = format!("# Doc\n\n{}", words);
        let time = reading_time_minutes(&md);
        assert!(time >= 1 && time <= 2);
    }

    #[test]
    fn test_plain_text_empty() {
        assert_eq!(to_plain_text(""), "");
        assert_eq!(to_plain_text("   "), "");
    }
}
