use sqlx::SqlitePool;

use crate::error::DbError;

/// Try to add a column to a table. Silently ignores if column already exists.
/// This is the standard pattern for incremental schema changes.
pub async fn try_add_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    column_type: &str,
    default: Option<&str>,
) -> Result<bool, DbError> {
    let sql = if let Some(def) = default {
        format!(
            "ALTER TABLE {} ADD COLUMN {} {} NOT NULL DEFAULT {}",
            table, column, column_type, def
        )
    } else {
        format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, column_type)
    };

    match sqlx::query(&sql).execute(pool).await {
        Ok(_) => {
            log::debug!("[uni-db] Added column {}.{}", table, column);
            Ok(true)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("duplicate column name") {
                Ok(false)
            } else {
                Err(DbError::Sqlx(e))
            }
        }
    }
}

/// Check if a table exists in the database
pub async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, DbError> {
    let result: Option<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type='table' AND name=?")
            .bind(table)
            .fetch_optional(pool)
            .await?;

    Ok(result.is_some())
}

/// Count rows in a table, optionally with a WHERE clause
pub async fn count(
    pool: &SqlitePool,
    table: &str,
    where_clause: Option<&str>,
) -> Result<i64, DbError> {
    let sql = if let Some(w) = where_clause {
        format!("SELECT COUNT(*) FROM {} WHERE {}", table, w)
    } else {
        format!("SELECT COUNT(*) FROM {}", table)
    };

    let count: (i64,) = sqlx::query_as(&sql).fetch_one(pool).await?;

    Ok(count.0)
}

/// Delete rows by ID column
pub async fn delete_by_id(pool: &SqlitePool, table: &str, id: &str) -> Result<u64, DbError> {
    let sql = format!("DELETE FROM {} WHERE id = ?", table);
    let result = sqlx::query(&sql).bind(id).execute(pool).await?;
    Ok(result.rows_affected())
}

/// Update a single column value by ID
pub async fn update_field(
    pool: &SqlitePool,
    table: &str,
    id: &str,
    column: &str,
    value: &str,
) -> Result<u64, DbError> {
    let sql = format!("UPDATE {} SET {} = ? WHERE id = ?", table, column);
    let result = sqlx::query(&sql).bind(value).bind(id).execute(pool).await?;
    Ok(result.rows_affected())
}

/// Update a single column to current timestamp and another column by ID
pub async fn update_field_with_timestamp(
    pool: &SqlitePool,
    table: &str,
    id: &str,
    column: &str,
    value: &str,
    timestamp_column: &str,
) -> Result<u64, DbError> {
    let now = uni_common::now_unix_secs();
    let sql = format!(
        "UPDATE {} SET {} = ?, {} = ? WHERE id = ?",
        table, column, timestamp_column
    );
    let result = sqlx::query(&sql)
        .bind(value)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Check if a row with given ID exists
pub async fn exists(pool: &SqlitePool, table: &str, id: &str) -> Result<bool, DbError> {
    let sql = format!("SELECT 1 FROM {} WHERE id = ? LIMIT 1", table);
    let result: Option<(i32,)> = sqlx::query_as(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(result.is_some())
}

/// Execute raw SQL, ignoring errors (for idempotent ALTER TABLE etc.)
/// Returns true if executed successfully, false if error was ignored.
pub async fn exec_ignore_error(pool: &SqlitePool, sql: &str) -> bool {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => true,
        Err(e) => {
            log::trace!(
                "[uni-db] Ignored error for '{}': {}",
                &sql[..sql.len().min(80)],
                e
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::{create_pool, DbConfig};
    use tempfile::TempDir;

    async fn test_pool_with_table() -> (SqlitePool, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let config = DbConfig::new(path.to_str().unwrap());
        let pool = create_pool(&config).await.unwrap();

        sqlx::query("CREATE TABLE items (id TEXT PRIMARY KEY, name TEXT NOT NULL, status TEXT DEFAULT 'active', updated_at INTEGER)")
            .execute(&pool)
            .await
            .unwrap();

        (pool, dir)
    }

    #[tokio::test]
    async fn test_table_exists() {
        let (pool, _dir) = test_pool_with_table().await;
        assert!(table_exists(&pool, "items").await.unwrap());
        assert!(!table_exists(&pool, "nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_try_add_column_new() {
        let (pool, _dir) = test_pool_with_table().await;
        let added = try_add_column(&pool, "items", "priority", "INTEGER", Some("0"))
            .await
            .unwrap();
        assert!(added);
    }

    #[tokio::test]
    async fn test_try_add_column_duplicate() {
        let (pool, _dir) = test_pool_with_table().await;
        try_add_column(&pool, "items", "priority", "INTEGER", Some("0"))
            .await
            .unwrap();
        let added = try_add_column(&pool, "items", "priority", "INTEGER", Some("0"))
            .await
            .unwrap();
        assert!(!added);
    }

    #[tokio::test]
    async fn test_count_empty() {
        let (pool, _dir) = test_pool_with_table().await;
        let c = count(&pool, "items", None).await.unwrap();
        assert_eq!(c, 0);
    }

    #[tokio::test]
    async fn test_count_with_rows() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query("INSERT INTO items (id, name) VALUES ('1', 'one'), ('2', 'two')")
            .execute(&pool)
            .await
            .unwrap();
        let c = count(&pool, "items", None).await.unwrap();
        assert_eq!(c, 2);
    }

    #[tokio::test]
    async fn test_count_with_where() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query(
            "INSERT INTO items (id, name, status) VALUES ('1', 'one', 'active'), ('2', 'two', 'done')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let c = count(&pool, "items", Some("status = 'active'"))
            .await
            .unwrap();
        assert_eq!(c, 1);
    }

    #[tokio::test]
    async fn test_delete_by_id() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query("INSERT INTO items (id, name) VALUES ('1', 'one')")
            .execute(&pool)
            .await
            .unwrap();

        let affected = delete_by_id(&pool, "items", "1").await.unwrap();
        assert_eq!(affected, 1);

        let c = count(&pool, "items", None).await.unwrap();
        assert_eq!(c, 0);
    }

    #[tokio::test]
    async fn test_delete_by_id_nonexistent() {
        let (pool, _dir) = test_pool_with_table().await;
        let affected = delete_by_id(&pool, "items", "999").await.unwrap();
        assert_eq!(affected, 0);
    }

    #[tokio::test]
    async fn test_update_field() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query("INSERT INTO items (id, name) VALUES ('1', 'one')")
            .execute(&pool)
            .await
            .unwrap();

        let affected = update_field(&pool, "items", "1", "name", "updated")
            .await
            .unwrap();
        assert_eq!(affected, 1);

        let name: (String,) = sqlx::query_as("SELECT name FROM items WHERE id = '1'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(name.0, "updated");
    }

    #[tokio::test]
    async fn test_exists() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query("INSERT INTO items (id, name) VALUES ('1', 'one')")
            .execute(&pool)
            .await
            .unwrap();

        assert!(exists(&pool, "items", "1").await.unwrap());
        assert!(!exists(&pool, "items", "999").await.unwrap());
    }

    #[tokio::test]
    async fn test_exec_ignore_error() {
        let (pool, _dir) = test_pool_with_table().await;

        let ok = exec_ignore_error(&pool, "CREATE TABLE temp (id TEXT)").await;
        assert!(ok);

        let fail = exec_ignore_error(&pool, "INVALID SQL HERE").await;
        assert!(!fail);
    }

    #[tokio::test]
    async fn test_update_field_with_timestamp() {
        let (pool, _dir) = test_pool_with_table().await;
        sqlx::query("INSERT INTO items (id, name, updated_at) VALUES ('1', 'one', 0)")
            .execute(&pool)
            .await
            .unwrap();

        update_field_with_timestamp(&pool, "items", "1", "name", "changed", "updated_at")
            .await
            .unwrap();

        let row: (String, i64) =
            sqlx::query_as("SELECT name, updated_at FROM items WHERE id = '1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row.0, "changed");
        assert!(row.1 > 0);
    }
}
