use std::path::Path;

use crate::types::*;

/// Convert PDF file to Markdown (basic text extraction).
/// Quality: Low — structure is not preserved.
pub fn convert_pdf(file_path: &Path) -> Result<ConversionResult, uni_common::UniError> {
    let bytes = std::fs::read(file_path).map_err(|e| uni_common::UniError::Io(e))?;

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| uni_common::UniError::Generic(format!("PDF extraction failed: {}", e)))?;

    if text.trim().is_empty() {
        return Err(uni_common::UniError::Generic(
            "PDF extraction produced no text (possibly scanned/image-only PDF)".to_string(),
        ));
    }

    // Clean up extracted text
    let cleaned = clean_pdf_text(&text);

    let title = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    Ok(ConversionResult {
        markdown: cleaned.clone(),
        title,
        metadata: ConversionMeta {
            original_format: "pdf".to_string(),
            pages: None, // pdf-extract doesn't provide page count easily
            word_count: count_words(&cleaned),
            has_images: false, // can't detect with pdf-extract
            has_tables: false, // can't detect with pdf-extract
            quality: ConversionQuality::Low,
            python_converted: false,
        },
    })
}

/// Clean up raw PDF extracted text:
/// - Normalize excessive whitespace
/// - Collapse multiple blank lines into two
/// - Trim each line
fn clean_pdf_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut blank_count = 0;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_pdf_text() {
        let raw = "  Line 1  \n\n\n\n\nLine 2\n  Line 3  ";
        let cleaned = clean_pdf_text(raw);
        assert_eq!(cleaned, "Line 1\n\n\nLine 2\nLine 3");
    }

    #[test]
    fn test_clean_pdf_text_preserves_content() {
        let raw = "Hello\nWorld";
        assert_eq!(clean_pdf_text(raw), "Hello\nWorld");
    }

    #[test]
    fn test_clean_pdf_text_empty() {
        assert_eq!(clean_pdf_text(""), "");
        assert_eq!(clean_pdf_text("   \n  \n  "), "");
    }

    #[test]
    fn test_convert_pdf_nonexistent_file() {
        let result = convert_pdf(Path::new("/nonexistent/file.pdf"));
        assert!(result.is_err());
    }

    // Note: integration test with real PDF would need a test fixture.
    // For now, test the text cleaning which is the main logic.
}
