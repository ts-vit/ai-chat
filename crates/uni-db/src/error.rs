use std::fmt;

#[derive(Debug)]
pub enum DbError {
    /// SQLx error wrapper
    Sqlx(sqlx::Error),
    /// Migration error
    Migration {
        version: u32,
        description: String,
        source: sqlx::Error,
    },
    /// Pool configuration error
    PoolConfig(String),
    /// Generic error with context
    Context {
        message: String,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Sqlx(e) => write!(f, "Database error: {}", e),
            DbError::Migration {
                version,
                description,
                source,
            } => {
                write!(
                    f,
                    "Migration V{} ({}) failed: {}",
                    version, description, source
                )
            }
            DbError::PoolConfig(msg) => write!(f, "Pool configuration error: {}", msg),
            DbError::Context { message, source } => {
                if let Some(src) = source {
                    write!(f, "{}: {}", message, src)
                } else {
                    write!(f, "{}", message)
                }
            }
        }
    }
}

impl std::error::Error for DbError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DbError::Sqlx(e) => Some(e),
            DbError::Migration { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        DbError::Sqlx(e)
    }
}

// Conversion to String for Tauri command compatibility
impl From<DbError> for String {
    fn from(e: DbError) -> Self {
        e.to_string()
    }
}
