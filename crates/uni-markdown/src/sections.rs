use crate::parse::{extract_headings, Heading};

/// A section of a markdown document, defined by headings.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Section {
    /// Section heading (None for content before first heading)
    pub heading: Option<Heading>,
    /// Full text content of this section (including heading line)
    pub content: String,
    /// Byte offset in source
    pub start_offset: usize,
    pub end_offset: usize,
    /// Heading hierarchy path (e.g. ["API Guide", "Auth", "OAuth2"])
    pub hierarchy: Vec<String>,
    /// Nesting depth (0 for top-level)
    pub depth: usize,
}

/// Split markdown into sections by heading level.
/// `max_level` controls the split granularity (1 = only H1, 2 = H1+H2, etc.)
/// Returns flat list of sections in document order.
pub fn split_sections(markdown: &str, max_level: u8) -> Vec<Section> {
    let headings = extract_headings(markdown);
    let relevant: Vec<&Heading> = headings.iter().filter(|h| h.level <= max_level).collect();

    if relevant.is_empty() {
        if !markdown.trim().is_empty() {
            return vec![Section {
                heading: None,
                content: markdown.to_string(),
                start_offset: 0,
                end_offset: markdown.len(),
                hierarchy: vec![],
                depth: 0,
            }];
        }
        return vec![];
    }

    let mut sections = Vec::new();

    // Content before first heading
    if relevant[0].start_offset > 0 {
        let pre_content = &markdown[..relevant[0].start_offset];
        if !pre_content.trim().is_empty() {
            sections.push(Section {
                heading: None,
                content: pre_content.to_string(),
                start_offset: 0,
                end_offset: relevant[0].start_offset,
                hierarchy: vec![],
                depth: 0,
            });
        }
    }

    let mut hierarchy_stack: Vec<(u8, String)> = Vec::new();

    for (i, heading) in relevant.iter().enumerate() {
        let start = heading.start_offset;
        let end = if i + 1 < relevant.len() {
            relevant[i + 1].start_offset
        } else {
            markdown.len()
        };

        let content = &markdown[start..end];

        // Update hierarchy stack
        while hierarchy_stack
            .last()
            .map_or(false, |(lvl, _)| *lvl >= heading.level)
        {
            hierarchy_stack.pop();
        }
        hierarchy_stack.push((heading.level, heading.text.clone()));

        let hierarchy: Vec<String> = hierarchy_stack.iter().map(|(_, t)| t.clone()).collect();
        let depth = (heading.level - 1) as usize;

        sections.push(Section {
            heading: Some((*heading).clone()),
            content: content.to_string(),
            start_offset: start,
            end_offset: end,
            hierarchy,
            depth,
        });
    }

    sections
}

/// Split markdown into sections by ALL heading levels (1-6).
pub fn split_all_sections(markdown: &str) -> Vec<Section> {
    split_sections(markdown, 6)
}

/// Find a section by heading text (case-insensitive partial match).
pub fn find_section(markdown: &str, heading_text: &str) -> Option<Section> {
    let sections = split_all_sections(markdown);
    let needle = heading_text.to_lowercase();
    sections.into_iter().find(|s| {
        s.heading
            .as_ref()
            .map_or(false, |h| h.text.to_lowercase().contains(&needle))
    })
}

/// Get heading hierarchy for a given byte offset in the document.
pub fn hierarchy_at_offset(markdown: &str, offset: usize) -> Vec<String> {
    let headings = extract_headings(markdown);
    let mut stack: Vec<(u8, String)> = Vec::new();

    for heading in &headings {
        if heading.start_offset > offset {
            break;
        }
        while stack
            .last()
            .map_or(false, |(lvl, _)| *lvl >= heading.level)
        {
            stack.pop();
        }
        stack.push((heading.level, heading.text.clone()));
    }

    stack.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sections_basic() {
        let md = "# Title\n\nIntro\n\n## Section 1\n\nContent 1\n\n## Section 2\n\nContent 2";
        let sections = split_sections(md, 2);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].heading.as_ref().unwrap().text, "Title");
        assert_eq!(sections[1].heading.as_ref().unwrap().text, "Section 1");
        assert_eq!(sections[2].heading.as_ref().unwrap().text, "Section 2");
    }

    #[test]
    fn test_split_sections_pre_heading_content() {
        let md = "Preamble text\n\n# Title\n\nContent";
        let sections = split_sections(md, 1);
        assert_eq!(sections.len(), 2);
        assert!(sections[0].heading.is_none());
        assert!(sections[0].content.contains("Preamble"));
        assert_eq!(sections[1].heading.as_ref().unwrap().text, "Title");
    }

    #[test]
    fn test_split_sections_no_headings() {
        let md = "Just plain text\nwith paragraphs";
        let sections = split_sections(md, 6);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].heading.is_none());
    }

    #[test]
    fn test_split_sections_hierarchy() {
        let md = "# Doc\n\n## Auth\n\n### OAuth\n\nContent\n\n## Settings\n\nMore";
        let sections = split_all_sections(md);
        let oauth = sections
            .iter()
            .find(|s| s.heading.as_ref().map_or(false, |h| h.text == "OAuth"))
            .unwrap();
        assert_eq!(oauth.hierarchy, vec!["Doc", "Auth", "OAuth"]);
    }

    #[test]
    fn test_split_sections_level_filter() {
        let md = "# Title\n\n## Section\n\n### Sub\n\nContent";
        let sections = split_sections(md, 1);
        assert_eq!(sections.len(), 1);
        assert!(sections[0].content.contains("### Sub"));
    }

    #[test]
    fn test_find_section() {
        let md = "# Title\n\nIntro\n\n## Auth\n\nUse tokens\n\n## Settings\n\nConfig";
        let section = find_section(md, "auth").unwrap();
        assert!(section.content.contains("Use tokens"));
    }

    #[test]
    fn test_find_section_not_found() {
        let md = "# Title\n\nContent";
        assert!(find_section(md, "nonexistent").is_none());
    }

    #[test]
    fn test_hierarchy_at_offset() {
        let md = "# Doc\n\n## Auth\n\n### OAuth\n\nContent here";
        let content_offset = md.find("Content here").unwrap();
        let hierarchy = hierarchy_at_offset(md, content_offset);
        assert_eq!(hierarchy, vec!["Doc", "Auth", "OAuth"]);
    }

    #[test]
    fn test_hierarchy_at_offset_before_headings() {
        let md = "Preamble\n\n# Title\n\nContent";
        let hierarchy = hierarchy_at_offset(md, 0);
        assert!(hierarchy.is_empty());
    }

    #[test]
    fn test_section_offsets() {
        let md = "# A\n\nText A\n\n# B\n\nText B";
        let sections = split_sections(md, 1);
        assert_eq!(sections.len(), 2);
        assert!(sections[0].end_offset <= sections[1].start_offset);
        assert_eq!(
            &md[sections[0].start_offset..sections[0].end_offset],
            sections[0].content
        );
    }
}
