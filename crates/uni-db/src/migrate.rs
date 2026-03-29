use sqlx::SqlitePool;

use crate::error::DbError;

/// A single database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Unique version number (must be sequential: 1, 2, 3, ...)
    pub version: u32,
    /// Short description (e.g. "create_chats_table")
    pub description: &'static str,
    /// SQL to execute. Can contain multiple statements separated by semicolons.
    pub sql: &'static str,
}

/// Run all pending migrations.
///
/// - Creates `_uni_migrations` table if not exists
/// - Checks which migrations have already been applied
/// - Runs unapplied migrations in order
/// - Records each successful migration in the table
/// - Stops and returns error on first failure
pub async fn run_migrations(pool: &SqlitePool, migrations: &[Migration]) -> Result<u32, DbError> {
    // Create migrations tracking table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _uni_migrations (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // Get already applied versions
    let applied: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _uni_migrations ORDER BY version")
            .fetch_all(pool)
            .await?;

    let applied_set: std::collections::HashSet<u32> =
        applied.iter().map(|v| *v as u32).collect();

    let mut count = 0u32;

    for migration in migrations {
        if applied_set.contains(&migration.version) {
            continue;
        }

        log::info!(
            "[uni-db] Running migration V{}: {}",
            migration.version,
            migration.description
        );

        // Execute migration SQL (may contain multiple statements)
        for statement in migration.sql.split(';') {
            let trimmed = statement.trim();
            if trimmed.is_empty() {
                continue;
            }
            sqlx::query(trimmed)
                .execute(pool)
                .await
                .map_err(|e| DbError::Migration {
                    version: migration.version,
                    description: migration.description.to_string(),
                    source: e,
                })?;
        }

        // Record migration
        let now = uni_common::now_unix_secs();
        sqlx::query(
            "INSERT INTO _uni_migrations (version, description, applied_at) VALUES (?, ?, ?)",
        )
        .bind(migration.version as i64)
        .bind(migration.description)
        .bind(now)
        .execute(pool)
        .await?;

        count += 1;
    }

    if count > 0 {
        log::info!("[uni-db] Applied {} migration(s)", count);
    }

    Ok(count)
}

/// Get the current migration version (highest applied version, or 0 if none)
pub async fn current_version(pool: &SqlitePool) -> Result<u32, DbError> {
    // Check if migrations table exists
    let exists: Option<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='_uni_migrations'",
    )
    .fetch_optional(pool)
    .await?;

    if exists.is_none() {
        return Ok(0);
    }

    let version: Option<i64> =
        sqlx::query_scalar("SELECT MAX(version) FROM _uni_migrations")
            .fetch_one(pool)
            .await?;

    Ok(version.unwrap_or(0) as u32)
}

/// Check that migrations are properly sequential (no gaps, no duplicates)
pub fn validate_migrations(migrations: &[Migration]) -> Result<(), DbError> {
    for (i, migration) in migrations.iter().enumerate() {
        let expected = (i + 1) as u32;
        if migration.version != expected {
            return Err(DbError::Context {
                message: format!(
                    "Migration version mismatch: expected V{}, got V{}",
                    expected, migration.version
                ),
                source: None,
            });
        }
    }

    // Check for duplicate versions
    let mut seen = std::collections::HashSet::new();
    for migration in migrations {
        if !seen.insert(migration.version) {
            return Err(DbError::Context {
                message: format!("Duplicate migration version: V{}", migration.version),
                source: None,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::{create_pool, DbConfig};
    use tempfile::TempDir;

    async fn test_pool() -> (SqlitePool, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let config = DbConfig::new(path.to_str().unwrap());
        let pool = create_pool(&config).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn test_run_migrations_empty() {
        let (pool, _dir) = test_pool().await;
        let count = run_migrations(&pool, &[]).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_run_migrations_creates_table() {
        let (pool, _dir) = test_pool().await;
        let migrations = vec![Migration {
            version: 1,
            description: "create_users",
            sql: "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
        }];
        let count = run_migrations(&pool, &migrations).await.unwrap();
        assert_eq!(count, 1);

        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='users'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
        assert_eq!(exists, Some("users".to_string()));
    }

    #[tokio::test]
    async fn test_run_migrations_idempotent() {
        let (pool, _dir) = test_pool().await;
        let migrations = vec![Migration {
            version: 1,
            description: "create_users",
            sql: "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
        }];

        let count1 = run_migrations(&pool, &migrations).await.unwrap();
        assert_eq!(count1, 1);

        let count2 = run_migrations(&pool, &migrations).await.unwrap();
        assert_eq!(count2, 0);
    }

    #[tokio::test]
    async fn test_run_migrations_incremental() {
        let (pool, _dir) = test_pool().await;

        let migrations_v1 = vec![Migration {
            version: 1,
            description: "create_users",
            sql: "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
        }];
        run_migrations(&pool, &migrations_v1).await.unwrap();

        let migrations_v2 = vec![
            Migration {
                version: 1,
                description: "create_users",
                sql: "CREATE TABLE users (id TEXT PRIMARY KEY, name TEXT NOT NULL)",
            },
            Migration {
                version: 2,
                description: "add_email",
                sql: "ALTER TABLE users ADD COLUMN email TEXT",
            },
        ];
        let count = run_migrations(&pool, &migrations_v2).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_run_migrations_multi_statement() {
        let (pool, _dir) = test_pool().await;
        let migrations = vec![Migration {
            version: 1,
            description: "create_tables",
            sql: "CREATE TABLE a (id TEXT PRIMARY KEY);
                  CREATE TABLE b (id TEXT PRIMARY KEY, a_id TEXT)",
        }];
        let count = run_migrations(&pool, &migrations).await.unwrap();
        assert_eq!(count, 1);

        let a_exists: Option<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name='a'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        let b_exists: Option<String> =
            sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name='b'")
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(a_exists.is_some());
        assert!(b_exists.is_some());
    }

    #[tokio::test]
    async fn test_run_migrations_failure_stops() {
        let (pool, _dir) = test_pool().await;
        let migrations = vec![
            Migration {
                version: 1,
                description: "create_users",
                sql: "CREATE TABLE users (id TEXT PRIMARY KEY)",
            },
            Migration {
                version: 2,
                description: "bad_sql",
                sql: "THIS IS NOT VALID SQL",
            },
            Migration {
                version: 3,
                description: "should_not_run",
                sql: "CREATE TABLE other (id TEXT PRIMARY KEY)",
            },
        ];
        let result = run_migrations(&pool, &migrations).await;
        assert!(result.is_err());

        let version = current_version(&pool).await.unwrap();
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn test_current_version_empty() {
        let (pool, _dir) = test_pool().await;
        let version = current_version(&pool).await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn test_current_version_after_migrations() {
        let (pool, _dir) = test_pool().await;
        let migrations = vec![
            Migration {
                version: 1,
                description: "v1",
                sql: "CREATE TABLE t1 (id TEXT)",
            },
            Migration {
                version: 2,
                description: "v2",
                sql: "CREATE TABLE t2 (id TEXT)",
            },
        ];
        run_migrations(&pool, &migrations).await.unwrap();
        let version = current_version(&pool).await.unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn test_validate_migrations_ok() {
        let migrations = vec![
            Migration {
                version: 1,
                description: "v1",
                sql: "",
            },
            Migration {
                version: 2,
                description: "v2",
                sql: "",
            },
        ];
        assert!(validate_migrations(&migrations).is_ok());
    }

    #[test]
    fn test_validate_migrations_gap() {
        let migrations = vec![
            Migration {
                version: 1,
                description: "v1",
                sql: "",
            },
            Migration {
                version: 3,
                description: "v3",
                sql: "",
            },
        ];
        assert!(validate_migrations(&migrations).is_err());
    }

    #[test]
    fn test_validate_migrations_duplicate() {
        let migrations = vec![
            Migration {
                version: 1,
                description: "v1",
                sql: "",
            },
            Migration {
                version: 1,
                description: "v1 dup",
                sql: "",
            },
        ];
        assert!(validate_migrations(&migrations).is_err());
    }
}
