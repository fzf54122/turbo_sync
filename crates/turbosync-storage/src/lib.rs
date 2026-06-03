use std::{path::Path, time::SystemTime};

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use turbosync_core::models::{
    CreateNodeRequest, CreateTaskRequest, Node, StatusResponse, SyncTask,
};
use uuid::Uuid;

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

pub async fn agent_status(pool: &SqlitePool, node_id: &str) -> Result<StatusResponse> {
    let enabled_nodes = sqlx::query_scalar("SELECT COUNT(*) FROM nodes WHERE enabled = 1")
        .fetch_one(pool)
        .await?;
    let enabled_tasks = sqlx::query_scalar("SELECT COUNT(*) FROM sync_tasks WHERE enabled = 1")
        .fetch_one(pool)
        .await?;
    let pending_events =
        sqlx::query_scalar("SELECT COUNT(*) FROM sync_events WHERE status = 'pending'")
            .fetch_one(pool)
            .await?;
    let last_sync_at = sqlx::query_scalar(
        "SELECT finished_at FROM sync_runs WHERE finished_at IS NOT NULL ORDER BY finished_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(StatusResponse {
        status: "running".to_owned(),
        node_id: node_id.to_owned(),
        enabled_nodes,
        enabled_tasks,
        pending_events,
        last_sync_at,
    })
}

pub async fn add_node(pool: &SqlitePool, request: &CreateNodeRequest) -> Result<Node> {
    let now = now_text();
    let node = Node {
        id: Uuid::new_v4().to_string(),
        name: request.name.clone(),
        endpoint: request.endpoint.clone(),
        public_key: request.public_key.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    sqlx::query(
        "INSERT INTO nodes (id, name, endpoint, public_key, enabled, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&node.id)
    .bind(&node.name)
    .bind(&node.endpoint)
    .bind(&node.public_key)
    .bind(bool_to_int(node.enabled))
    .bind(&node.created_at)
    .bind(&node.updated_at)
    .execute(pool)
    .await?;

    Ok(node)
}

pub async fn list_nodes(pool: &SqlitePool) -> Result<Vec<Node>> {
    let rows = sqlx::query(
        "SELECT id, name, endpoint, public_key, enabled, created_at, updated_at FROM nodes ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_node).collect())
}

pub async fn remove_node(pool: &SqlitePool, node_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM nodes WHERE id = $1")
        .bind(node_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn add_sync_task(pool: &SqlitePool, request: &CreateTaskRequest) -> Result<SyncTask> {
    let now = now_text();
    let task = SyncTask {
        id: Uuid::new_v4().to_string(),
        name: request.name.clone(),
        source_path: request.source_path.clone(),
        target_node_id: request.target_node_id.clone(),
        target_path: request.target_path.clone(),
        direction: request.direction.clone(),
        delete_mode: request.delete_mode.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    sqlx::query(
        "INSERT INTO sync_tasks (id, name, source_path, target_node_id, target_path, direction, delete_mode, enabled, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&task.id)
    .bind(&task.name)
    .bind(&task.source_path)
    .bind(&task.target_node_id)
    .bind(&task.target_path)
    .bind(&task.direction)
    .bind(&task.delete_mode)
    .bind(bool_to_int(task.enabled))
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .execute(pool)
    .await?;

    Ok(task)
}

pub async fn list_sync_tasks(pool: &SqlitePool) -> Result<Vec<SyncTask>> {
    let rows = sqlx::query(
        "SELECT id, name, source_path, target_node_id, target_path, direction, delete_mode, enabled, created_at, updated_at FROM sync_tasks ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_sync_task).collect())
}

pub async fn remove_sync_task(pool: &SqlitePool, task_id: &str) -> Result<bool> {
    let result = sqlx::query("DELETE FROM sync_tasks WHERE id = $1")
        .bind(task_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

fn row_to_node(row: sqlx::sqlite::SqliteRow) -> Node {
    Node {
        id: row.get("id"),
        name: row.get("name"),
        endpoint: row.get("endpoint"),
        public_key: row.get("public_key"),
        enabled: int_to_bool(row.get("enabled")),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_sync_task(row: sqlx::sqlite::SqliteRow) -> SyncTask {
    SyncTask {
        id: row.get("id"),
        name: row.get("name"),
        source_path: row.get("source_path"),
        target_node_id: row.get("target_node_id"),
        target_path: row.get("target_path"),
        direction: row.get("direction"),
        delete_mode: row.get("delete_mode"),
        enabled: int_to_bool(row.get("enabled")),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn now_text() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
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

    #[tokio::test]
    async fn node_and_task_crud_updates_status_counts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("state/turbosync.db");
        let pool = initialize_database(&db_path).await.unwrap();

        let node = add_node(
            &pool,
            &CreateNodeRequest {
                name: "nas".to_owned(),
                endpoint: "127.0.0.1:38746".to_owned(),
                public_key: None,
            },
        )
        .await
        .unwrap();
        let task = add_sync_task(
            &pool,
            &CreateTaskRequest::one_way(
                "docs".to_owned(),
                "/tmp/docs".to_owned(),
                node.id.clone(),
                "/backup/docs".to_owned(),
            ),
        )
        .await
        .unwrap();

        let status = agent_status(&pool, "local-node").await.unwrap();
        assert_eq!(status.enabled_nodes, 1);
        assert_eq!(status.enabled_tasks, 1);
        assert_eq!(list_nodes(&pool).await.unwrap(), vec![node.clone()]);
        assert_eq!(list_sync_tasks(&pool).await.unwrap(), vec![task.clone()]);

        assert!(remove_sync_task(&pool, &task.id).await.unwrap());
        assert!(remove_node(&pool, &node.id).await.unwrap());
        assert!(list_nodes(&pool).await.unwrap().is_empty());
        assert!(list_sync_tasks(&pool).await.unwrap().is_empty());
    }
}
