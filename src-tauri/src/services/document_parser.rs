// Document text extraction for Knowledge Base indexing

use std::path::Path;

/// Extract text content from a document file based on its MIME type.
/// HTML files are stripped of tags (legacy behavior).
pub fn extract_text(file_path: &str, mime_type: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    match mime_type {
        "text/html" => {
            let html = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read HTML file: {}", e))?;
            Ok(super::web_content::strip_html_tags(&html))
        }
        "application/pdf" => extract_pdf(path),
        // All text-based formats: plain, markdown, csv, json, xml, code, etc.
        m if m.starts_with("text/") || m == "application/json" || m == "application/xml"
            || m == "application/javascript" => {
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
        }
        _ => Err(format!("Unsupported format: {}", mime_type)),
    }
}

/// Extract raw text content, preserving HTML structure for the chunker to handle.
/// HTML files are returned as-is (the chunker converts headings to markdown).
pub fn extract_text_raw(file_path: &str, mime_type: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    match mime_type {
        "text/html" => {
            // Return raw HTML — chunker::chunk_html() handles conversion
            std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read HTML file: {}", e))
        }
        // All other types: same as extract_text
        _ => extract_text(file_path, mime_type),
    }
}

fn extract_pdf(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read PDF: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract PDF text: {}", e))
}
