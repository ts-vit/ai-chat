use sqlx::SqlitePool;

use crate::error::DbError;

/// Execute a closure within a database transaction.
///
/// If the closure returns Ok, the transaction is committed.
/// If it returns Err, the transaction is rolled back.
pub async fn with_transaction<'a, F, T, Fut>(pool: &'a SqlitePool, f: F) -> Result<T, DbError>
where
    F: FnOnce(sqlx::Transaction<'a, sqlx::Sqlite>) -> Fut,
    Fut: std::future::Future<
        Output = Result<(T, sqlx::Transaction<'a, sqlx::Sqlite>), DbError>,
    >,
{
    let tx = pool.begin().await?;
    let (result, tx): (T, sqlx::Transaction<'a, sqlx::Sqlite>) = f(tx).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DbError;
    use crate::pool::{create_pool, DbConfig};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transaction_commit() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = create_pool(&DbConfig::new(path.to_str().unwrap()))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE items (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        let result = with_transaction(&pool, |mut tx| async move {
            sqlx::query("INSERT INTO items (id) VALUES ('1')")
                .execute(&mut *tx)
                .await
                .map_err(DbError::from)?;
            Ok(("inserted", tx))
        })
        .await
        .unwrap();
        assert_eq!(result, "inserted");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 1);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = create_pool(&DbConfig::new(path.to_str().unwrap()))
            .await
            .unwrap();
        sqlx::query("CREATE TABLE items (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();

        let result: Result<&str, DbError> = with_transaction(&pool, |mut tx| async move {
            sqlx::query("INSERT INTO items (id) VALUES ('1')")
                .execute(&mut *tx)
                .await
                .map_err(DbError::from)?;
            Err(DbError::Context {
                message: "intentional error".to_string(),
                source: None,
            })
        })
        .await;
        assert!(result.is_err());

        // Row should NOT be persisted
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM items")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }
}
