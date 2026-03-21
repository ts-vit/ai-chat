use std::path::Path;

use crate::types::*;

/// Convert a plain text file to Markdown (passthrough — wrap as-is).
pub fn convert_txt(content: &str, file_name: Option<&str>) -> ConversionResult {
    let title = file_name.map(|n| {
        // Strip extension for title
        Path::new(n)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(n)
            .to_string()
    });

    ConversionResult {
        markdown: content.to_string(),
        title,
        metadata: ConversionMeta {
            original_format: "txt".to_string(),
            pages: None,
            word_count: count_words(content),
            has_images: false,
            has_tables: false,
            quality: ConversionQuality::High,
            python_converted: false,
        },
    }
}

/// Convert a Markdown file (passthrough — already in target format).
pub fn convert_md(content: &str, file_name: Option<&str>) -> ConversionResult {
    let title = extract_md_title(content).or_else(|| {
        file_name.map(|n| {
            Path::new(n)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(n)
                .to_string()
        })
    });

    ConversionResult {
        markdown: content.to_string(),
        title,
        metadata: ConversionMeta {
            original_format: "md".to_string(),
            pages: None,
            word_count: count_words(content),
            has_images: content.contains("!["),
            has_tables: content.contains("| "),
            quality: ConversionQuality::High,
            python_converted: false,
        },
    }
}

/// Extract title from first # heading in markdown.
/// Properly handles YAML frontmatter (--- delimited blocks).
fn extract_md_title(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    let mut frontmatter_started = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Handle frontmatter
        if trimmed == "---" {
            if !frontmatter_started {
                // Opening ---
                frontmatter_started = true;
                in_frontmatter = true;
                continue;
            } else if in_frontmatter {
                // Closing ---
                in_frontmatter = false;
                continue;
            }
        }

        // Skip lines inside frontmatter
        if in_frontmatter {
            continue;
        }

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Found # heading
        if trimmed.starts_with("# ") {
            return Some(trimmed[2..].trim().to_string());
        }

        // First non-empty, non-heading line after frontmatter — no title
        break;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_txt() {
        let result = convert_txt("Hello world\nLine 2", Some("notes.txt"));
        assert_eq!(result.markdown, "Hello world\nLine 2");
        assert_eq!(result.title.as_deref(), Some("notes"));
        assert_eq!(result.metadata.original_format, "txt");
        assert_eq!(result.metadata.quality, ConversionQuality::High);
        assert_eq!(result.metadata.word_count, 4);
    }

    #[test]
    fn test_convert_md_with_title() {
        let md = "# My Document\n\nSome content here.";
        let result = convert_md(md, None);
        assert_eq!(result.title.as_deref(), Some("My Document"));
        assert_eq!(result.metadata.original_format, "md");
    }

    #[test]
    fn test_convert_md_with_frontmatter() {
        let md = "---\ntitle: Test\n---\n# Real Title\n\nContent";
        let result = convert_md(md, None);
        assert_eq!(result.title.as_deref(), Some("Real Title"));
    }

    #[test]
    fn test_convert_md_no_title_fallback_filename() {
        let md = "Just some text without heading";
        let result = convert_md(md, Some("readme.md"));
        assert_eq!(result.title.as_deref(), Some("readme"));
    }

    #[test]
    fn test_convert_md_detects_images_and_tables() {
        let md = "# Doc\n![photo](img.png)\n| A | B |\n| - | - |";
        let result = convert_md(md, None);
        assert!(result.metadata.has_images);
        assert!(result.metadata.has_tables);
    }

    #[test]
    fn test_extract_md_title() {
        assert_eq!(extract_md_title("# Hello"), Some("Hello".to_string()));
        assert_eq!(extract_md_title("## Not H1"), None);
        assert_eq!(extract_md_title("no heading"), None);
        assert_eq!(extract_md_title(""), None);
        assert_eq!(
            extract_md_title("---\n---\n# After Frontmatter"),
            Some("After Frontmatter".to_string())
        );
    }
}
