//! Python-backed document conversion via `uni-python`.

use std::path::{Path, PathBuf};

use uni_common::{CancellationToken, UniError};
use uni_python::{ExecutionRequest, ProgressInfo, PythonConfig, PythonEnvironment, PythonExecutor};

use crate::types::{ConversionMeta, ConversionQuality, ConversionResult};

const MANIFEST_TOML: &str = include_str!("../python/converter/manifest.toml");
const MAIN_PY: &str = include_str!("../python/converter/main.py");
const PDF_CONVERTER_PY: &str = include_str!("../python/converter/pdf_converter.py");
const DOCX_CONVERTER_PY: &str = include_str!("../python/converter/docx_converter.py");
const XLSX_CONVERTER_PY: &str = include_str!("../python/converter/xlsx_converter.py");
const PPTX_CONVERTER_PY: &str = include_str!("../python/converter/pptx_converter.py");
const EPUB_CONVERTER_PY: &str = include_str!("../python/converter/epub_converter.py");
const REQUIREMENTS_TXT: &str = include_str!("../python/converter/requirements.txt");

const SCRIPT_FILES: &[(&str, &str)] = &[
    ("manifest.toml", MANIFEST_TOML),
    ("main.py", MAIN_PY),
    ("pdf_converter.py", PDF_CONVERTER_PY),
    ("docx_converter.py", DOCX_CONVERTER_PY),
    ("xlsx_converter.py", XLSX_CONVERTER_PY),
    ("pptx_converter.py", PPTX_CONVERTER_PY),
    ("epub_converter.py", EPUB_CONVERTER_PY),
    ("requirements.txt", REQUIREMENTS_TXT),
];

/// Extract bundled converter scripts to `{target_dir}/converter/`.
/// Returns path to the converter script directory.
pub async fn extract_scripts(target_dir: &Path) -> Result<PathBuf, UniError> {
    let script_dir = target_dir.join("converter");
    tokio::fs::create_dir_all(&script_dir)
        .await
        .map_err(UniError::Io)?;

    for (name, content) in SCRIPT_FILES {
        let file_path = script_dir.join(name);
        let needs_write = if file_path.exists() {
            let existing = tokio::fs::read_to_string(&file_path)
                .await
                .unwrap_or_default();
            existing != *content
        } else {
            true
        };
        if needs_write {
            tokio::fs::write(&file_path, content)
                .await
                .map_err(UniError::Io)?;
        }
    }

    log::info!("Extracted converter scripts to {:?}", script_dir);
    Ok(script_dir)
}

/// Convert a file using the Python converter stack (`uni-python` + venv + bundled script).
pub async fn convert_with_python(
    file_path: &Path,
    format: &str,
    python_base_dir: &Path,
    cancel_token: CancellationToken,
    on_progress: Option<Box<dyn Fn(ProgressInfo) + Send>>,
) -> Result<ConversionResult, UniError> {
    let lib_dir = uni_python::ensure_bridge(python_base_dir).await?;

    let scripts_dir = python_base_dir.join("scripts").join("bundled");
    let converter_dir = extract_scripts(&scripts_dir).await?;

    let config = PythonConfig::new(python_base_dir.to_path_buf());
    let env = PythonEnvironment::new(config.clone());
    let status = env.ensure_ready().await?;

    let venv_python = status
        .python_executable()
        .ok_or_else(|| UniError::Generic("Python not ready".to_string()))?
        .clone();

    let req_path = converter_dir.join("requirements.txt");
    if req_path.exists() {
        let dep_status = env.check_dependencies(&req_path).await?;
        if !dep_status.missing.is_empty() {
            log::info!(
                "Installing converter dependencies: {:?}",
                dep_status.missing
            );
            env.install_requirements(&req_path).await?;
        }
    }

    let entry_path = converter_dir.join("main.py");
    let executor = PythonExecutor::new(venv_python, python_base_dir.to_path_buf());

    let result = executor
        .execute(
            &entry_path,
            ExecutionRequest {
                method: "convert".to_string(),
                params: serde_json::json!({
                    "input_path": file_path.to_string_lossy(),
                    "format": format,
                    "options": {},
                }),
                timeout_secs: 120,
                input_files: vec![file_path.to_path_buf()],
                env_vars: vec![(
                    "UNI_LIB_DIR".to_string(),
                    lib_dir.to_string_lossy().into_owned(),
                )],
            },
            cancel_token,
            on_progress,
        )
        .await?;

    if !result.success {
        return Err(UniError::Generic(
            result
                .error
                .unwrap_or_else(|| "Python conversion failed".to_string()),
        ));
    }

    let data = result
        .result
        .ok_or_else(|| UniError::Generic("Python converter returned no result".to_string()))?;

    parse_python_result(data)
}

fn parse_python_result(data: serde_json::Value) -> Result<ConversionResult, UniError> {
    let markdown = data
        .get("markdown")
        .and_then(|v| v.as_str())
        .ok_or_else(|| UniError::Generic("Missing 'markdown' in converter result".to_string()))?
        .to_string();

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let metadata_val = data
        .get("metadata")
        .ok_or_else(|| UniError::Generic("Missing 'metadata' in converter result".to_string()))?;

    let quality_str = metadata_val
        .get("quality")
        .and_then(|v| v.as_str())
        .unwrap_or("medium");

    let quality = match quality_str {
        "high" => ConversionQuality::High,
        "low" => ConversionQuality::Low,
        _ => ConversionQuality::Medium,
    };

    Ok(ConversionResult {
        markdown,
        title,
        metadata: ConversionMeta {
            original_format: metadata_val
                .get("original_format")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            pages: metadata_val
                .get("pages")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
            word_count: metadata_val
                .get("word_count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            has_images: metadata_val
                .get("has_images")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            has_tables: metadata_val
                .get("has_tables")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            quality,
            python_converted: true,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_result_success() {
        let data = serde_json::json!({
            "markdown": "# Hello\n\nWorld",
            "title": "Hello",
            "metadata": {
                "original_format": "pdf",
                "pages": 5,
                "word_count": 2,
                "has_images": true,
                "has_tables": false,
                "quality": "high",
                "python_converted": true,
            }
        });
        let result = parse_python_result(data).unwrap();
        assert_eq!(result.markdown, "# Hello\n\nWorld");
        assert_eq!(result.title.as_deref(), Some("Hello"));
        assert_eq!(result.metadata.quality, ConversionQuality::High);
        assert_eq!(result.metadata.pages, Some(5));
        assert!(result.metadata.has_images);
        assert!(result.metadata.python_converted);
    }

    #[test]
    fn test_parse_python_result_missing_markdown() {
        let data = serde_json::json!({"title": "Test"});
        assert!(parse_python_result(data).is_err());
    }

    #[test]
    fn test_parse_python_result_minimal() {
        let data = serde_json::json!({
            "markdown": "content",
            "metadata": {
                "original_format": "docx",
                "quality": "medium"
            }
        });
        let result = parse_python_result(data).unwrap();
        assert_eq!(result.markdown, "content");
        assert!(result.title.is_none());
        assert_eq!(result.metadata.quality, ConversionQuality::Medium);
    }

    #[tokio::test]
    async fn test_extract_scripts() {
        let base = std::env::temp_dir().join("uni-converter-extract-test");
        let _ = tokio::fs::remove_dir_all(&base).await;
        let dir = extract_scripts(&base).await.unwrap();
        assert!(dir.join("main.py").exists());
        assert!(dir.join("manifest.toml").exists());
        assert!(dir.join("pdf_converter.py").exists());
        assert!(dir.join("requirements.txt").exists());
        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}
