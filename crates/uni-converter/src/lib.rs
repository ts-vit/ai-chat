pub mod csv_conv;
pub mod html;
pub mod markdown;
pub mod pdf;
pub mod python;
pub mod rtf;
pub mod types;
pub mod web;
pub mod youtube;

pub use python::{convert_with_python, extract_scripts};
pub use types::{ConversionMeta, ConversionQuality, ConversionResult};

use std::path::Path;

/// Convert a file to Markdown based on its MIME type.
///
/// Supported MIME types:
/// - text/plain → passthrough
/// - text/markdown → passthrough
/// - text/html → HTML→MD (htmd)
/// - text/csv → CSV→MD table
/// - application/pdf → PDF text extraction
/// - application/rtf, text/rtf → RTF text extraction
///
/// Unsupported formats return an error (DOCX, XLSX, PPTX, EPUB
/// will be handled by Python in F8b-2).
pub fn convert_file(
    file_path: &Path,
    mime_type: &str,
) -> Result<ConversionResult, uni_common::UniError> {
    let file_name = file_path.file_name().and_then(|n| n.to_str());

    match mime_type {
        "text/plain" => {
            let content =
                std::fs::read_to_string(file_path).map_err(|e| uni_common::UniError::Io(e))?;
            Ok(markdown::convert_txt(&content, file_name))
        }
        "text/markdown" | "text/x-markdown" => {
            let content =
                std::fs::read_to_string(file_path).map_err(|e| uni_common::UniError::Io(e))?;
            Ok(markdown::convert_md(&content, file_name))
        }
        "text/html" | "application/xhtml+xml" => {
            let content =
                std::fs::read_to_string(file_path).map_err(|e| uni_common::UniError::Io(e))?;
            html::convert_html(&content, file_name)
        }
        "text/csv" => {
            let content =
                std::fs::read_to_string(file_path).map_err(|e| uni_common::UniError::Io(e))?;
            csv_conv::convert_csv(&content, file_name)
        }
        "application/pdf" => pdf::convert_pdf(file_path),
        "application/rtf" | "text/rtf" => {
            let content =
                std::fs::read_to_string(file_path).map_err(|e| uni_common::UniError::Io(e))?;
            Ok(rtf::convert_rtf(&content, file_name))
        }
        _ => Err(uni_common::UniError::Generic(format!(
            "Unsupported format: {}. DOCX/XLSX/PPTX/EPUB require Python (see F8b-2)",
            mime_type
        ))),
    }
}

/// Convert a file to Markdown with Python-backed formats where applicable.
///
/// For formats supported by Rust (TXT, MD, HTML, CSV, RTF): uses Rust directly.
/// For PDF: tries Python (PyMuPDF) → falls back to Rust (`pdf-extract`).
/// For DOCX, XLSX, PPTX, EPUB: requires Python.
pub async fn convert_file_with_python(
    file_path: &Path,
    mime_type: &str,
    python_base_dir: &Path,
    cancel_token: uni_common::CancellationToken,
    on_progress: Option<Box<dyn Fn(uni_python::ProgressInfo) + Send>>,
) -> Result<ConversionResult, uni_common::UniError> {
    match mime_type {
        "text/plain"
        | "text/markdown"
        | "text/x-markdown"
        | "text/html"
        | "application/xhtml+xml"
        | "text/csv"
        | "application/rtf"
        | "text/rtf" => {
            return convert_file(file_path, mime_type);
        }
        _ => {}
    }

    if mime_type == "application/pdf" {
        match python::convert_with_python(
            file_path,
            "pdf",
            python_base_dir,
            cancel_token.clone(),
            on_progress,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(e) => {
                log::warn!("Python PDF conversion failed, falling back to Rust: {}", e);
                return pdf::convert_pdf(file_path);
            }
        }
    }

    let format = match mime_type {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "application/epub+zip" => "epub",
        _ => {
            return Err(uni_common::UniError::Generic(format!(
                "Unsupported format: {}",
                mime_type
            )));
        }
    };

    python::convert_with_python(
        file_path,
        format,
        python_base_dir,
        cancel_token,
        on_progress,
    )
    .await
}

/// Convert a URL to Markdown.
pub async fn convert_url(
    client: &reqwest::Client,
    url: &str,
) -> Result<ConversionResult, uni_common::UniError> {
    web::convert_url(client, url).await
}

/// Convert a YouTube video to Markdown (via subtitles).
pub async fn convert_youtube(
    client: &reqwest::Client,
    url: &str,
) -> Result<ConversionResult, uni_common::UniError> {
    youtube::convert_youtube(client, url).await
}

/// Convert raw text input to Markdown (passthrough with metadata).
pub fn convert_text(text: &str, name: Option<&str>) -> ConversionResult {
    markdown::convert_txt(text, name)
}

/// Guess MIME type from file extension.
/// Returns None for unknown extensions.
pub fn guess_mime_type(file_path: &Path) -> Option<&'static str> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("txt") => Some("text/plain"),
        Some("md" | "markdown") => Some("text/markdown"),
        Some("html" | "htm") => Some("text/html"),
        Some("csv") => Some("text/csv"),
        Some("pdf") => Some("application/pdf"),
        Some("rtf") => Some("application/rtf"),
        // Future: Python-handled formats
        Some("docx") => {
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        }
        Some("xlsx") => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        Some("pptx") => {
            Some("application/vnd.openxmlformats-officedocument.presentationml.presentation")
        }
        Some("epub") => Some("application/epub+zip"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type(Path::new("doc.txt")), Some("text/plain"));
        assert_eq!(guess_mime_type(Path::new("doc.md")), Some("text/markdown"));
        assert_eq!(guess_mime_type(Path::new("page.html")), Some("text/html"));
        assert_eq!(guess_mime_type(Path::new("data.csv")), Some("text/csv"));
        assert_eq!(
            guess_mime_type(Path::new("doc.pdf")),
            Some("application/pdf")
        );
        assert_eq!(
            guess_mime_type(Path::new("doc.rtf")),
            Some("application/rtf")
        );
        assert_eq!(guess_mime_type(Path::new("unknown.xyz")), None);
        // Case insensitive
        assert_eq!(
            guess_mime_type(Path::new("DOC.PDF")),
            Some("application/pdf")
        );
    }

    #[test]
    fn test_convert_file_txt() {
        let dir = std::env::temp_dir().join("uni-converter-test-txt");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("test.txt");
        std::fs::write(&file, "Hello world").unwrap();

        let result = convert_file(&file, "text/plain").unwrap();
        assert_eq!(result.markdown, "Hello world");
        assert_eq!(result.metadata.original_format, "txt");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_convert_file_md() {
        let dir = std::env::temp_dir().join("uni-converter-test-md");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("readme.md");
        std::fs::write(&file, "# Title\n\nContent").unwrap();

        let result = convert_file(&file, "text/markdown").unwrap();
        assert_eq!(result.title.as_deref(), Some("Title"));
        assert_eq!(result.metadata.original_format, "md");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_convert_file_html() {
        let dir = std::env::temp_dir().join("uni-converter-test-html");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("page.html");
        std::fs::write(&file, "<h1>Hello</h1><p>World</p>").unwrap();

        let result = convert_file(&file, "text/html").unwrap();
        assert!(result.markdown.contains("# Hello"));
        assert_eq!(result.metadata.original_format, "html");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_convert_file_csv() {
        let dir = std::env::temp_dir().join("uni-converter-test-csv");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("data.csv");
        std::fs::write(&file, "Name,Age\nAlice,30").unwrap();

        let result = convert_file(&file, "text/csv").unwrap();
        assert!(result.markdown.contains("| Name | Age |"));
        assert!(result.metadata.has_tables);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_convert_file_rtf() {
        let dir = std::env::temp_dir().join("uni-converter-test-rtf");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("doc.rtf");
        std::fs::write(&file, r"{\rtf1\ansi Hello RTF world}").unwrap();

        let result = convert_file(&file, "application/rtf").unwrap();
        assert!(result.markdown.contains("Hello RTF world"));
        assert_eq!(result.metadata.quality, ConversionQuality::Low);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_convert_file_unsupported() {
        let result = convert_file(
            Path::new("test.docx"),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_convert_text() {
        let result = convert_text("Just some text", Some("note"));
        assert_eq!(result.markdown, "Just some text");
        assert_eq!(result.title.as_deref(), Some("note"));
    }
}
