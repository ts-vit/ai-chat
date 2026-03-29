//! uni-db — SQLite database layer for UNI Framework
//!
//! Provides pool creation, migration system, and CRUD helpers
//! as a thin wrapper over sqlx.

mod error;
mod helpers;
mod migrate;
mod pool;
mod transaction;

pub use error::DbError;
pub use helpers::{
    count, delete_by_id, exec_ignore_error, exists, table_exists, try_add_column, update_field,
    update_field_with_timestamp,
};
pub use migrate::{current_version, run_migrations, validate_migrations, Migration};
pub use pool::{create_pool, DbConfig};
pub use transaction::with_transaction;

// Re-export sqlx types that consumers commonly need
pub use sqlx::sqlite::SqliteRow;
pub use sqlx::{Row, SqlitePool};
