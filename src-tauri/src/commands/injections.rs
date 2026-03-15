use std::path::PathBuf;

/// Read a text file from disk for {{file:path}} injection.
/// Returns file contents as String. Limits to 100KB to prevent accidental huge files.
#[tauri::command]
pub async fn read_file_contents(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);

    if !file_path.exists() {
        return Err(format!("File not found: {}", path));
    }

    if !file_path.is_file() {
        return Err(format!("Not a file: {}", path));
    }

    let metadata =
        std::fs::metadata(&file_path).map_err(|e| format!("Cannot read file metadata: {}", e))?;

    if metadata.len() > 100 * 1024 {
        return Err(format!(
            "File too large: {} bytes (max 100KB). Path: {}",
            metadata.len(),
            path
        ));
    }

    std::fs::read_to_string(&file_path).map_err(|e| format!("Cannot read file: {}", e))
}
