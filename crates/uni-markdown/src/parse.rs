use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// A heading found in the document.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Heading {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text content
    pub text: String,
    /// Byte offset in source where heading starts
    pub start_offset: usize,
    /// Byte offset in source where heading ends
    pub end_offset: usize,
}

/// A code block found in the document.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeBlock {
    /// Language tag (e.g. "rust", "python"), empty if unspecified
    pub language: String,
    /// Code content
    pub content: String,
    /// Byte offset in source
    pub start_offset: usize,
    pub end_offset: usize,
}

/// A link found in the document.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Link {
    /// Link URL
    pub url: String,
    /// Link title (alt text)
    pub title: String,
    /// Link display text
    pub text: String,
}

/// Parse options for pulldown-cmark.
pub(crate) fn parser_options() -> Options {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    opts
}

/// Convert pulldown-cmark HeadingLevel to u8.
fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Extract all headings from markdown text.
pub fn extract_headings(markdown: &str) -> Vec<Heading> {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut headings = Vec::new();
    let mut current_heading: Option<(u8, String, usize)> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current_heading = Some((heading_level_to_u8(level), String::new(), range.start));
            }
            Event::Text(text) | Event::Code(text) if current_heading.is_some() => {
                if let Some((_, ref mut heading_text, _)) = current_heading {
                    heading_text.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, text, start)) = current_heading.take() {
                    headings.push(Heading {
                        level,
                        text,
                        start_offset: start,
                        end_offset: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

/// Extract all code blocks from markdown text.
pub fn extract_code_blocks(markdown: &str) -> Vec<CodeBlock> {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut blocks = Vec::new();
    let mut current_block: Option<(String, String, usize)> = None;

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) => lang.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                current_block = Some((lang, String::new(), range.start));
            }
            Event::Text(text) if current_block.is_some() => {
                if let Some((_, ref mut content, _)) = current_block {
                    content.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((lang, content, start)) = current_block.take() {
                    blocks.push(CodeBlock {
                        language: lang,
                        content,
                        start_offset: start,
                        end_offset: range.end,
                    });
                }
            }
            _ => {}
        }
    }

    blocks
}

/// Extract all links from markdown text.
pub fn extract_links(markdown: &str) -> Vec<Link> {
    let parser = Parser::new_ext(markdown, parser_options());
    let mut links = Vec::new();
    let mut current_link: Option<(String, String, String)> = None;

    for (event, _range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::Link { dest_url, title, .. }) => {
                current_link = Some((dest_url.to_string(), title.to_string(), String::new()));
            }
            Event::Text(text) if current_link.is_some() => {
                if let Some((_, _, ref mut link_text)) = current_link {
                    link_text.push_str(&text);
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((url, title, text)) = current_link.take() {
                    links.push(Link { url, title, text });
                }
            }
            _ => {}
        }
    }

    links
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_headings_basic() {
        let md = "# Title\n\nSome text\n\n## Section 1\n\nContent\n\n### Subsection\n\nMore";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].text, "Title");
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].text, "Section 1");
        assert_eq!(headings[2].level, 3);
        assert_eq!(headings[2].text, "Subsection");
    }

    #[test]
    fn test_extract_headings_empty() {
        let md = "Just plain text\nwithout headings";
        let headings = extract_headings(md);
        assert!(headings.is_empty());
    }

    #[test]
    fn test_headings_inside_code_block_ignored() {
        let md = "# Real Heading\n\n```\n# Not a heading\n## Also not\n```\n\n## Real Section";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].text, "Real Heading");
        assert_eq!(headings[1].text, "Real Section");
    }

    #[test]
    fn test_extract_code_blocks() {
        let md = "Some text\n\n```rust\nfn main() {}\n```\n\n```python\nprint('hi')\n```";
        let blocks = extract_code_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "rust");
        assert!(blocks[0].content.contains("fn main()"));
        assert_eq!(blocks[1].language, "python");
    }

    #[test]
    fn test_extract_code_blocks_no_language() {
        let md = "```\nplain code\n```";
        let blocks = extract_code_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "");
    }

    #[test]
    fn test_extract_links() {
        let md =
            "Check [example](https://example.com) and [docs](https://docs.rs \"Documentation\")";
        let links = extract_links(md);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].url, "https://example.com");
        assert_eq!(links[0].text, "example");
        assert_eq!(links[1].url, "https://docs.rs");
        assert_eq!(links[1].title, "Documentation");
    }

    #[test]
    fn test_heading_offsets() {
        let md = "# Title\n\nText\n\n## Section";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].start_offset, 0);
        assert!(headings[0].end_offset > 0);
        assert!(headings[1].start_offset > headings[0].end_offset);
    }

    #[test]
    fn test_heading_with_inline_code() {
        let md = "## The `Config` struct\n\nContent";
        let headings = extract_headings(md);
        assert_eq!(headings.len(), 1);
        assert_eq!(headings[0].text, "The Config struct");
    }
}
