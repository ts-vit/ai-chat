// Document text extraction for Knowledge Base indexing

use std::path::Path;

/// Extract text content from a document file based on its MIME type.
pub fn extract_text(file_path: &str, mime_type: &str) -> Result<String, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    match mime_type {
        "text/plain" | "text/csv" | "text/x-rst" | "application/json" | "application/xml" => {
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
        }
        "text/markdown" => {
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
        }
        "text/html" => {
            let html = std::fs::read_to_string(path)
                .map_err(|e| format!("Failed to read HTML file: {}", e))?;
            Ok(super::web_content::strip_html_tags(&html))
        }
        "application/pdf" => extract_pdf(path),
        _ => Err(format!("Unsupported format: {}", mime_type)),
    }
}

fn extract_pdf(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read PDF: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract PDF text: {}", e))
}
