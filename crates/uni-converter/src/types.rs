use serde::{Deserialize, Serialize};

/// Quality of conversion — how much structure was preserved
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConversionQuality {
    /// Almost lossless: TXT, MD, HTML — structure fully preserved
    High,
    /// Partially preserved: DOCX, EPUB — some formatting lost
    Medium,
    /// Approximate: PDF, PPTX — text extracted, structure guessed
    Low,
}

impl std::fmt::Display for ConversionQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

/// Metadata about the conversion process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionMeta {
    /// Original format (e.g. "pdf", "html", "csv")
    pub original_format: String,
    /// Number of pages (for PDF, PPTX)
    pub pages: Option<usize>,
    /// Word count of the resulting markdown
    pub word_count: usize,
    /// Whether images were detected but not converted
    pub has_images: bool,
    /// Whether tables were detected
    pub has_tables: bool,
    /// Conversion quality
    pub quality: ConversionQuality,
    /// Whether this was converted by Python (true) or Rust (false)
    pub python_converted: bool,
}

/// Result of document conversion
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// The converted Markdown content
    pub markdown: String,
    /// Extracted document title (if found)
    pub title: Option<String>,
    /// Conversion metadata
    pub metadata: ConversionMeta,
}

impl ConversionResult {
    /// Generate YAML frontmatter for the converted document
    pub fn frontmatter(&self, source: &str) -> String {
        let mut fm = String::from("---\n");
        if let Some(ref title) = self.title {
            fm.push_str(&format!("title: \"{}\"\n", title.replace('"', "\\\"")));
        }
        fm.push_str(&format!("source: \"{}\"\n", source.replace('"', "\\\"")));
        fm.push_str(&format!("format: \"{}\"\n", self.metadata.original_format));
        fm.push_str(&format!("quality: \"{}\"\n", self.metadata.quality));
        if let Some(pages) = self.metadata.pages {
            fm.push_str(&format!("pages: {}\n", pages));
        }
        if self.metadata.has_images {
            fm.push_str("has_images: true\n");
        }
        if self.metadata.has_tables {
            fm.push_str("has_tables: true\n");
        }
        fm.push_str("---\n");
        fm
    }
}

/// Count words in text
pub fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Quality warning message for converted documents
pub fn quality_warning(quality: ConversionQuality, format: &str) -> Option<String> {
    match quality {
        ConversionQuality::Low => Some(format!(
            "> ⚠️ This document was converted from {}. Some formatting may be lost.\n",
            format.to_uppercase()
        )),
        ConversionQuality::Medium => Some(format!(
            "> ℹ️ This document was converted from {}. Minor formatting differences are possible.\n",
            format.to_uppercase()
        )),
        ConversionQuality::High => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_quality_display() {
        assert_eq!(ConversionQuality::High.to_string(), "high");
        assert_eq!(ConversionQuality::Medium.to_string(), "medium");
        assert_eq!(ConversionQuality::Low.to_string(), "low");
    }

    #[test]
    fn test_count_words() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("hello"), 1);
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("  spaced  out  "), 2);
        assert_eq!(count_words("Привет мир"), 2);
    }

    #[test]
    fn test_quality_warning() {
        assert!(quality_warning(ConversionQuality::High, "txt").is_none());
        assert!(quality_warning(ConversionQuality::Low, "pdf")
            .unwrap()
            .contains("PDF"));
        assert!(quality_warning(ConversionQuality::Medium, "docx")
            .unwrap()
            .contains("DOCX"));
    }

    #[test]
    fn test_frontmatter() {
        let result = ConversionResult {
            markdown: "# Test".to_string(),
            title: Some("My Document".to_string()),
            metadata: ConversionMeta {
                original_format: "pdf".to_string(),
                pages: Some(42),
                word_count: 1000,
                has_images: true,
                has_tables: false,
                quality: ConversionQuality::Low,
                python_converted: false,
            },
        };
        let fm = result.frontmatter("file:document.pdf");
        assert!(fm.starts_with("---\n"));
        assert!(fm.ends_with("---\n"));
        assert!(fm.contains("title: \"My Document\""));
        assert!(fm.contains("format: \"pdf\""));
        assert!(fm.contains("quality: \"low\""));
        assert!(fm.contains("pages: 42"));
        assert!(fm.contains("has_images: true"));
        assert!(!fm.contains("has_tables"));
    }

    #[test]
    fn test_frontmatter_escapes_quotes() {
        let result = ConversionResult {
            markdown: String::new(),
            title: Some("Document \"with quotes\"".to_string()),
            metadata: ConversionMeta {
                original_format: "txt".to_string(),
                pages: None,
                word_count: 0,
                has_images: false,
                has_tables: false,
                quality: ConversionQuality::High,
                python_converted: false,
            },
        };
        let fm = result.frontmatter("test");
        assert!(fm.contains("\\\"with quotes\\\""));
    }
}
