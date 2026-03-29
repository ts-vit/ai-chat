use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

use crate::error::DbError;

/// Configuration for database pool
pub struct DbConfig {
    /// Path to SQLite database file
    pub path: String,
    /// Maximum number of connections (default: 5)
    pub max_connections: u32,
    /// Enable WAL journal mode (default: true)
    pub wal_mode: bool,
    /// Busy timeout in milliseconds (default: 5000)
    pub busy_timeout_ms: u64,
    /// Create database file if it doesn't exist (default: true)
    pub create_if_missing: bool,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            max_connections: 5,
            wal_mode: true,
            busy_timeout_ms: 5000,
            create_if_missing: true,
        }
    }
}

impl DbConfig {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            ..Default::default()
        }
    }

    pub fn with_max_connections(mut self, max: u32) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_busy_timeout(mut self, ms: u64) -> Self {
        self.busy_timeout_ms = ms;
        self
    }
}

/// Create a configured SQLite connection pool.
///
/// - Creates parent directories if needed
/// - Sets WAL journal mode
/// - Sets busy timeout
/// - Returns configured SqlitePool
pub async fn create_pool(config: &DbConfig) -> Result<SqlitePool, DbError> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&config.path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DbError::PoolConfig(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    let mode = if config.create_if_missing {
        "rwc"
    } else {
        "rw"
    };
    let url = format!("sqlite:{}?mode={}", config.path, mode);

    let options = SqliteConnectOptions::from_str(&url)
        .map_err(|e| DbError::PoolConfig(format!("Invalid database URL: {}", e)))?
        .busy_timeout(std::time::Duration::from_millis(config.busy_timeout_ms));

    let pool = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .connect_with(options)
        .await
        .map_err(|e| DbError::PoolConfig(format!("Failed to connect: {}", e)))?;

    // Set pragmas
    if config.wal_mode {
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;
    }

    log::debug!("Database pool created: {}", config.path);

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_pool_new_db() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let config = DbConfig::new(path.to_str().unwrap());
        let pool = create_pool(&config).await.unwrap();

        // Verify WAL mode
        let mode: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(mode.0.to_lowercase(), "wal");
    }

    #[tokio::test]
    async fn test_create_pool_creates_parent_dirs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("subdir").join("nested").join("test.db");
        let config = DbConfig::new(path.to_str().unwrap());
        let pool = create_pool(&config).await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_create_pool_custom_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let config = DbConfig::new(path.to_str().unwrap())
            .with_max_connections(2)
            .with_busy_timeout(1000);
        let pool = create_pool(&config).await;
        assert!(pool.is_ok());
    }
}
