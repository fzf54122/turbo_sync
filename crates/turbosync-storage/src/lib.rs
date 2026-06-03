use std::path::Path;

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};

pub async fn initialize_database(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create database directory {}", parent.display()))?;
    }

    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .with_context(|| format!("failed to open SQLite database {}", db_path.display()))?;

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initialize_database_creates_expected_tables() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("state/turbosync.db");

        let pool = initialize_database(&db_path).await.unwrap();
        let table_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('nodes', 'sync_tasks', 'file_index', 'sync_events', 'sync_runs', 'sync_operations')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(table_count, 6);
        assert!(db_path.exists());
    }
}
