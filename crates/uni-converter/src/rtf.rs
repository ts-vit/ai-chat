use regex::Regex;

use crate::types::*;

/// Convert RTF content to plain text (basic extraction via regex stripping).
/// Quality: Low — all formatting is lost.
pub fn convert_rtf(content: &str, file_name: Option<&str>) -> ConversionResult {
    let text = strip_rtf(content);

    let title = file_name.map(|n| {
        std::path::Path::new(n)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(n)
            .to_string()
    });

    ConversionResult {
        markdown: text.clone(),
        title,
        metadata: ConversionMeta {
            original_format: "rtf".to_string(),
            pages: None,
            word_count: count_words(&text),
            has_images: false,
            has_tables: false,
            quality: ConversionQuality::Low,
            python_converted: false,
        },
    }
}

/// Strip RTF control words and groups, extract plain text.
fn strip_rtf(rtf: &str) -> String {
    // Remove RTF header
    let re_groups = Regex::new(
        r"(?s)\{\\(?:fonttbl|colortbl|stylesheet|info|header|footer|pict)[^{}]*(?:\{[^{}]*\}[^{}]*)*\}",
    )
    .unwrap();
    let cleaned = re_groups.replace_all(rtf, "");

    // Remove control words (e.g. \par, \b0, \fs24)
    let re_control = Regex::new(r"\\[a-zA-Z]+[-]?\d*\s?").unwrap();
    let cleaned = re_control.replace_all(&cleaned, "");

    // Remove remaining braces
    let cleaned = cleaned.replace('{', "").replace('}', "");

    // Remove escaped special chars
    let cleaned = cleaned
        .replace("\\'92", "'")
        .replace("\\'93", "\u{201C}") // left double quote
        .replace("\\'94", "\u{201D}") // right double quote
        .replace("\\'96", "\u{2013}") // en dash
        .replace("\\'97", "\u{2014}") // em dash
        .replace("\\'e9", "é")
        .replace("\\'e8", "è")
        .replace("\\\\", "\\")
        .replace("\\{", "{")
        .replace("\\}", "}");

    // Normalize whitespace
    let re_ws = Regex::new(r"[ \t]+").unwrap();
    let cleaned = re_ws.replace_all(&cleaned, " ");

    // Collapse blank lines
    let re_blank = Regex::new(r"\n{3,}").unwrap();
    let cleaned = re_blank.replace_all(&cleaned, "\n\n");

    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_rtf_basic() {
        let rtf = r"{\rtf1\ansi Hello world}";
        let text = strip_rtf(rtf);
        assert!(text.contains("Hello world"));
    }

    #[test]
    fn test_strip_rtf_control_words() {
        let rtf = r"{\rtf1 \b Bold\b0  text \i italic\i0 }";
        let text = strip_rtf(rtf);
        assert!(text.contains("Bold"));
        assert!(text.contains("text"));
        assert!(text.contains("italic"));
        assert!(!text.contains("\\b"));
    }

    #[test]
    fn test_strip_rtf_special_chars() {
        let rtf = r"{\rtf1 don\'92t stop}";
        let text = strip_rtf(rtf);
        assert!(text.contains("don't"));
    }

    #[test]
    fn test_strip_rtf_empty() {
        assert_eq!(strip_rtf(""), "");
    }

    #[test]
    fn test_convert_rtf() {
        let rtf = r"{\rtf1\ansi Simple document text}";
        let result = convert_rtf(rtf, Some("notes.rtf"));
        assert!(result.markdown.contains("Simple document text"));
        assert_eq!(result.title.as_deref(), Some("notes"));
        assert_eq!(result.metadata.quality, ConversionQuality::Low);
        assert_eq!(result.metadata.original_format, "rtf");
    }
}
