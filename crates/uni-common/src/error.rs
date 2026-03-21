use thiserror::Error;

#[derive(Debug, Error)]
pub enum UniError {
    #[error("{0}")]
    Generic(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Cancelled")]
    Cancelled,
}

impl From<String> for UniError {
    fn from(s: String) -> Self {
        UniError::Generic(s)
    }
}

impl From<&str> for UniError {
    fn from(s: &str) -> Self {
        UniError::Generic(s.to_string())
    }
}

// Conversion to String for compatibility with current Tauri Result<..., String>
impl From<UniError> for String {
    fn from(e: UniError) -> String {
        e.to_string()
    }
}
