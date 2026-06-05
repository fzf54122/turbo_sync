use std::{path::Path, time::SystemTime};

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Row, SqlitePool,
};
use turbosync_core::models::{
    CreateNodeRequest, CreateTaskRequest, FileIndexEntry, Node, StatusResponse, SyncEvent,
    SyncOperation, SyncRun, SyncTask, UpdateTaskRequest,
};
use uuid::Uuid;

pub struct FinishSyncRun {
    pub status: String,
    pub files_scanned: i64,
    pub files_changed: i64,
    pub files_failed: i64,
    pub bytes_sent: i64,
    pub error_message: Option<String>,
}

pub struct CreateSyncOperation {
    pub relative_path: String,
    pub operation_kind: String,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub error_message: Option<String>,
}

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
        health_status: "unchecked".to_owned(),
        health_message: None,
        last_checked_at: None,
        created_at: now.clone(),
        updated_at: now,
    };

    sqlx::query(
        "INSERT INTO nodes (id, name, endpoint, public_key, enabled, health_status, health_message, last_checked_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind(&node.id)
    .bind(&node.name)
    .bind(&node.endpoint)
    .bind(&node.public_key)
    .bind(bool_to_int(node.enabled))
    .bind(&node.health_status)
    .bind(&node.health_message)
    .bind(&node.last_checked_at)
    .bind(&node.created_at)
    .bind(&node.updated_at)
    .execute(pool)
    .await?;

    Ok(node)
}

pub async fn list_nodes(pool: &SqlitePool) -> Result<Vec<Node>> {
    let rows = sqlx::query(
        "SELECT id, name, endpoint, public_key, enabled, health_status, health_message, last_checked_at, created_at, updated_at FROM nodes ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_node).collect())
}

pub async fn get_node(pool: &SqlitePool, node_id: &str) -> Result<Option<Node>> {
    let row = sqlx::query(
        "SELECT id, name, endpoint, public_key, enabled, health_status, health_message, last_checked_at, created_at, updated_at FROM nodes WHERE id = $1",
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(row_to_node))
}

pub async fn update_node_health(
    pool: &SqlitePool,
    node_id: &str,
    health_status: &str,
    health_message: Option<&str>,
) -> Result<()> {
    let now = now_text();
    sqlx::query(
        "UPDATE nodes SET health_status = $1, health_message = $2, last_checked_at = $3, updated_at = $3 WHERE id = $4",
    )
    .bind(health_status)
    .bind(health_message)
    .bind(&now)
    .bind(node_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn count_sync_tasks_for_node(pool: &SqlitePool, node_id: &str) -> Result<i64> {
    let count = sqlx::query_scalar("SELECT COUNT(*) FROM sync_tasks WHERE target_node_id = $1")
        .bind(node_id)
        .fetch_one(pool)
        .await?;

    Ok(count)
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
        conflict_mode: request.conflict_mode.clone(),
        enabled: true,
        created_at: now.clone(),
        updated_at: now,
    };

    sqlx::query(
        "INSERT INTO sync_tasks (id, name, source_path, target_node_id, target_path, direction, delete_mode, conflict_mode, enabled, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&task.id)
    .bind(&task.name)
    .bind(&task.source_path)
    .bind(&task.target_node_id)
    .bind(&task.target_path)
    .bind(&task.direction)
    .bind(&task.delete_mode)
    .bind(&task.conflict_mode)
    .bind(bool_to_int(task.enabled))
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .execute(pool)
    .await?;

    Ok(task)
}

pub async fn get_sync_task(pool: &SqlitePool, task_id: &str) -> Result<Option<SyncTask>> {
    let row = sqlx::query(
        "SELECT id, name, source_path, target_node_id, target_path, direction, delete_mode, conflict_mode, enabled, created_at, updated_at FROM sync_tasks WHERE id = $1",
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(row_to_sync_task))
}

pub async fn list_sync_tasks(pool: &SqlitePool) -> Result<Vec<SyncTask>> {
    let rows = sqlx::query(
        "SELECT id, name, source_path, target_node_id, target_path, direction, delete_mode, conflict_mode, enabled, created_at, updated_at FROM sync_tasks ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_sync_task).collect())
}

pub async fn update_sync_task(
    pool: &SqlitePool,
    task_id: &str,
    updates: &UpdateTaskRequest,
) -> Result<Option<SyncTask>> {
    let existing = get_sync_task(pool, task_id).await?;
    let Some(task) = existing else {
        return Ok(None);
    };

    let now = now_text();
    let name = updates.name.clone().unwrap_or(task.name);
    let source_path = updates.source_path.clone().unwrap_or(task.source_path);
    let target_path = updates.target_path.clone().unwrap_or(task.target_path);
    let enabled = updates.enabled.unwrap_or(task.enabled);

    sqlx::query(
        "UPDATE sync_tasks SET name = $1, source_path = $2, target_path = $3, enabled = $4, updated_at = $5 WHERE id = $6",
    )
    .bind(&name)
    .bind(&source_path)
    .bind(&target_path)
    .bind(bool_to_int(enabled))
    .bind(&now)
    .bind(task_id)
    .execute(pool)
    .await?;

    get_sync_task(pool, task_id).await
}

pub async fn remove_sync_task(pool: &SqlitePool, task_id: &str) -> Result<bool> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "DELETE FROM sync_operations WHERE sync_run_id IN (SELECT id FROM sync_runs WHERE task_id = $1)",
    )
    .bind(task_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM sync_runs WHERE task_id = $1")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM sync_events WHERE task_id = $1")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM file_index WHERE task_id = $1")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    let result = sqlx::query("DELETE FROM sync_tasks WHERE id = $1")
        .bind(task_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(result.rows_affected() > 0)
}

pub async fn list_file_index_for_task(
    pool: &SqlitePool,
    task_id: &str,
) -> Result<Vec<FileIndexEntry>> {
    let rows = sqlx::query(
        "SELECT task_id, relative_path, file_kind, size_bytes, modified_at, content_hash, deleted, last_seen_at, last_synced_at, updated_at FROM file_index WHERE task_id = $1 ORDER BY relative_path ASC",
    )
    .bind(task_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_file_index_entry).collect())
}

pub async fn upsert_file_index_entries(
    pool: &SqlitePool,
    entries: &[FileIndexEntry],
) -> Result<()> {
    for entry in entries {
        sqlx::query(
            "INSERT INTO file_index (task_id, relative_path, file_kind, size_bytes, modified_at, content_hash, deleted, last_seen_at, last_synced_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) ON CONFLICT(task_id, relative_path) DO UPDATE SET file_kind = excluded.file_kind, size_bytes = excluded.size_bytes, modified_at = excluded.modified_at, content_hash = excluded.content_hash, deleted = excluded.deleted, last_seen_at = excluded.last_seen_at, updated_at = excluded.updated_at",
        )
        .bind(&entry.task_id)
        .bind(&entry.relative_path)
        .bind(&entry.file_kind)
        .bind(entry.size_bytes)
        .bind(&entry.modified_at)
        .bind(&entry.content_hash)
        .bind(bool_to_int(entry.deleted))
        .bind(&entry.last_seen_at)
        .bind(&entry.last_synced_at)
        .bind(&entry.updated_at)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn mark_file_index_deleted(
    pool: &SqlitePool,
    task_id: &str,
    relative_paths: &[String],
) -> Result<()> {
    let now = now_text();

    for relative_path in relative_paths {
        sqlx::query(
            "UPDATE file_index SET deleted = 1, updated_at = $1 WHERE task_id = $2 AND relative_path = $3",
        )
        .bind(&now)
        .bind(task_id)
        .bind(relative_path)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn set_file_index_synced(
    pool: &SqlitePool,
    task_id: &str,
    relative_paths: &[String],
    synced_at: &str,
) -> Result<()> {
    for relative_path in relative_paths {
        sqlx::query(
            "UPDATE file_index SET last_synced_at = $1, updated_at = $1 WHERE task_id = $2 AND relative_path = $3",
        )
        .bind(synced_at)
        .bind(task_id)
        .bind(relative_path)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn create_sync_run(
    pool: &SqlitePool,
    task_id: Option<&str>,
    trigger_kind: &str,
) -> Result<SyncRun> {
    let now = now_text();
    let run = SyncRun {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.map(str::to_owned),
        trigger_kind: trigger_kind.to_owned(),
        status: "running".to_owned(),
        started_at: now,
        finished_at: None,
        files_scanned: 0,
        files_changed: 0,
        files_failed: 0,
        bytes_sent: 0,
        error_message: None,
    };

    sqlx::query(
        "INSERT INTO sync_runs (id, task_id, trigger_kind, status, started_at, finished_at, files_scanned, files_changed, files_failed, bytes_sent, error_message) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&run.id)
    .bind(&run.task_id)
    .bind(&run.trigger_kind)
    .bind(&run.status)
    .bind(&run.started_at)
    .bind(&run.finished_at)
    .bind(run.files_scanned)
    .bind(run.files_changed)
    .bind(run.files_failed)
    .bind(run.bytes_sent)
    .bind(&run.error_message)
    .execute(pool)
    .await?;

    Ok(run)
}

pub async fn finish_sync_run(
    pool: &SqlitePool,
    run_id: &str,
    update: &FinishSyncRun,
) -> Result<SyncRun> {
    let finished_at = now_text();

    sqlx::query(
        "UPDATE sync_runs SET status = $1, finished_at = $2, files_scanned = $3, files_changed = $4, files_failed = $5, bytes_sent = $6, error_message = $7 WHERE id = $8",
    )
    .bind(&update.status)
    .bind(&finished_at)
    .bind(update.files_scanned)
    .bind(update.files_changed)
    .bind(update.files_failed)
    .bind(update.bytes_sent)
    .bind(&update.error_message)
    .bind(run_id)
    .execute(pool)
    .await?;

    get_sync_run(pool, run_id).await
}

pub async fn list_recent_sync_runs(pool: &SqlitePool, limit: i64) -> Result<Vec<SyncRun>> {
    let rows = sqlx::query(
        "SELECT id, task_id, trigger_kind, status, started_at, finished_at, files_scanned, files_changed, files_failed, bytes_sent, error_message FROM sync_runs ORDER BY started_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_sync_run).collect())
}

pub async fn mark_running_sync_runs_failed(pool: &SqlitePool, reason: &str) -> Result<u64> {
    let finished_at = now_text();
    let result = sqlx::query(
        "UPDATE sync_runs SET status = 'failed', finished_at = $1, files_failed = CASE WHEN files_failed = 0 THEN 1 ELSE files_failed END, error_message = $2 WHERE status = 'running'",
    )
    .bind(&finished_at)
    .bind(reason)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn add_sync_operation(
    pool: &SqlitePool,
    sync_run_id: &str,
    request: &CreateSyncOperation,
) -> Result<SyncOperation> {
    let now = now_text();
    let operation = SyncOperation {
        id: Uuid::new_v4().to_string(),
        sync_run_id: sync_run_id.to_owned(),
        relative_path: request.relative_path.clone(),
        operation_kind: request.operation_kind.clone(),
        status: request.status.clone(),
        size_bytes: request.size_bytes,
        error_message: request.error_message.clone(),
        created_at: now.clone(),
        finished_at: Some(now),
    };

    sqlx::query(
        "INSERT INTO sync_operations (id, sync_run_id, relative_path, operation_kind, status, size_bytes, error_message, created_at, finished_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&operation.id)
    .bind(&operation.sync_run_id)
    .bind(&operation.relative_path)
    .bind(&operation.operation_kind)
    .bind(&operation.status)
    .bind(operation.size_bytes)
    .bind(&operation.error_message)
    .bind(&operation.created_at)
    .bind(&operation.finished_at)
    .execute(pool)
    .await?;

    Ok(operation)
}

pub async fn list_sync_operations_for_run(
    pool: &SqlitePool,
    sync_run_id: &str,
) -> Result<Vec<SyncOperation>> {
    let rows = sqlx::query(
        "SELECT id, sync_run_id, relative_path, operation_kind, status, size_bytes, error_message, created_at, finished_at FROM sync_operations WHERE sync_run_id = $1 ORDER BY created_at ASC",
    )
    .bind(sync_run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_sync_operation).collect())
}

pub async fn add_sync_event(
    pool: &SqlitePool,
    task_id: &str,
    relative_path: &str,
    event_kind: &str,
) -> Result<SyncEvent> {
    let now = now_text();
    let event = SyncEvent {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        relative_path: relative_path.to_owned(),
        event_kind: event_kind.to_owned(),
        status: "pending".to_owned(),
        error_message: None,
        created_at: now,
        processed_at: None,
    };

    sqlx::query(
        "INSERT INTO sync_events (id, task_id, relative_path, event_kind, status, error_message, created_at, processed_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&event.id)
    .bind(&event.task_id)
    .bind(&event.relative_path)
    .bind(&event.event_kind)
    .bind(&event.status)
    .bind(&event.error_message)
    .bind(&event.created_at)
    .bind(&event.processed_at)
    .execute(pool)
    .await?;

    Ok(event)
}

pub async fn mark_sync_event_processed(pool: &SqlitePool, event_id: &str) -> Result<()> {
    let now = now_text();
    sqlx::query("UPDATE sync_events SET status = 'processed', processed_at = $1 WHERE id = $2")
        .bind(&now)
        .bind(event_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_pending_sync_events(pool: &SqlitePool, limit: i64) -> Result<Vec<SyncEvent>> {
    let rows = sqlx::query(
        "SELECT id, task_id, relative_path, event_kind, status, error_message, created_at, processed_at FROM sync_events WHERE status = 'pending' ORDER BY created_at ASC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(row_to_sync_event).collect())
}

async fn get_sync_run(pool: &SqlitePool, run_id: &str) -> Result<SyncRun> {
    let row = sqlx::query(
        "SELECT id, task_id, trigger_kind, status, started_at, finished_at, files_scanned, files_changed, files_failed, bytes_sent, error_message FROM sync_runs WHERE id = $1",
    )
    .bind(run_id)
    .fetch_one(pool)
    .await?;

    Ok(row_to_sync_run(row))
}

fn row_to_node(row: sqlx::sqlite::SqliteRow) -> Node {
    Node {
        id: row.get("id"),
        name: row.get("name"),
        endpoint: row.get("endpoint"),
        public_key: row.get("public_key"),
        enabled: int_to_bool(row.get("enabled")),
        health_status: row.get("health_status"),
        health_message: row.get("health_message"),
        last_checked_at: row.get("last_checked_at"),
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
        conflict_mode: row.get("conflict_mode"),
        enabled: int_to_bool(row.get("enabled")),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_file_index_entry(row: sqlx::sqlite::SqliteRow) -> FileIndexEntry {
    FileIndexEntry {
        task_id: row.get("task_id"),
        relative_path: row.get("relative_path"),
        file_kind: row.get("file_kind"),
        size_bytes: row.get("size_bytes"),
        modified_at: row.get("modified_at"),
        content_hash: row.get("content_hash"),
        deleted: int_to_bool(row.get("deleted")),
        last_seen_at: row.get("last_seen_at"),
        last_synced_at: row.get("last_synced_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_sync_run(row: sqlx::sqlite::SqliteRow) -> SyncRun {
    SyncRun {
        id: row.get("id"),
        task_id: row.get("task_id"),
        trigger_kind: row.get("trigger_kind"),
        status: row.get("status"),
        started_at: row.get("started_at"),
        finished_at: row.get("finished_at"),
        files_scanned: row.get("files_scanned"),
        files_changed: row.get("files_changed"),
        files_failed: row.get("files_failed"),
        bytes_sent: row.get("bytes_sent"),
        error_message: row.get("error_message"),
    }
}

fn row_to_sync_event(row: sqlx::sqlite::SqliteRow) -> SyncEvent {
    SyncEvent {
        id: row.get("id"),
        task_id: row.get("task_id"),
        relative_path: row.get("relative_path"),
        event_kind: row.get("event_kind"),
        status: row.get("status"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        processed_at: row.get("processed_at"),
    }
}

fn row_to_sync_operation(row: sqlx::sqlite::SqliteRow) -> SyncOperation {
    SyncOperation {
        id: row.get("id"),
        sync_run_id: row.get("sync_run_id"),
        relative_path: row.get("relative_path"),
        operation_kind: row.get("operation_kind"),
        status: row.get("status"),
        size_bytes: row.get("size_bytes"),
        error_message: row.get("error_message"),
        created_at: row.get("created_at"),
        finished_at: row.get("finished_at"),
    }
}

fn bool_to_int(value: bool) -> i64 {
    i64::from(value)
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

pub fn now_text() -> String {
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
        assert_eq!(node.health_status, "unchecked");
        assert!(node.health_message.is_none());
        assert!(node.last_checked_at.is_none());

        update_node_health(&pool, &node.id, "failed", Some("connection refused"))
            .await
            .unwrap();
        let checked_node = get_node(&pool, &node.id).await.unwrap().unwrap();
        assert_eq!(checked_node.health_status, "failed");
        assert_eq!(
            checked_node.health_message.as_deref(),
            Some("connection refused")
        );
        assert!(checked_node.last_checked_at.is_some());

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
        assert_eq!(list_nodes(&pool).await.unwrap(), vec![checked_node.clone()]);
        assert_eq!(list_sync_tasks(&pool).await.unwrap(), vec![task.clone()]);
        assert_eq!(
            get_sync_task(&pool, &task.id).await.unwrap(),
            Some(task.clone())
        );

        assert!(remove_sync_task(&pool, &task.id).await.unwrap());
        assert!(remove_node(&pool, &node.id).await.unwrap());
        assert!(list_nodes(&pool).await.unwrap().is_empty());
        assert!(list_sync_tasks(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn file_index_upsert_updates_existing_entry() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;
        let mut entry = file_entry(&task.id, "notes/a.txt", "hash-a", 12);

        upsert_file_index_entries(&pool, &[entry.clone()])
            .await
            .unwrap();
        entry.content_hash = Some("hash-b".to_owned());
        entry.size_bytes = Some(18);
        upsert_file_index_entries(&pool, &[entry]).await.unwrap();

        let entries = list_file_index_for_task(&pool, &task.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content_hash.as_deref(), Some("hash-b"));
        assert_eq!(entries[0].size_bytes, Some(18));
        assert!(!entries[0].deleted);
    }

    #[tokio::test]
    async fn file_index_marks_missing_entries_deleted_without_removing_rows() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;
        let entry = file_entry(&task.id, "notes/a.txt", "hash-a", 12);

        upsert_file_index_entries(&pool, &[entry]).await.unwrap();
        mark_file_index_deleted(&pool, &task.id, &["notes/a.txt".to_owned()])
            .await
            .unwrap();

        let entries = list_file_index_for_task(&pool, &task.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].deleted);
    }

    #[tokio::test]
    async fn sync_run_lifecycle_records_counts() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;
        let run = create_sync_run(&pool, Some(&task.id), "manual_sync")
            .await
            .unwrap();

        let finished = finish_sync_run(
            &pool,
            &run.id,
            &FinishSyncRun {
                status: "success".to_owned(),
                files_scanned: 3,
                files_changed: 2,
                files_failed: 1,
                bytes_sent: 42,
                error_message: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(finished.status, "success");
        assert_eq!(finished.files_scanned, 3);
        assert_eq!(finished.files_changed, 2);
        assert_eq!(finished.files_failed, 1);
        assert_eq!(finished.bytes_sent, 42);
        assert!(finished.finished_at.is_some());
        assert_eq!(
            list_recent_sync_runs(&pool, 10).await.unwrap(),
            vec![finished]
        );
    }

    #[tokio::test]
    async fn sync_operations_are_listed_by_run() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;
        let run = create_sync_run(&pool, Some(&task.id), "manual_sync")
            .await
            .unwrap();

        let operation = add_sync_operation(
            &pool,
            &run.id,
            &CreateSyncOperation {
                relative_path: "notes/a.txt".to_owned(),
                operation_kind: "create_file".to_owned(),
                status: "success".to_owned(),
                size_bytes: Some(12),
                error_message: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            list_sync_operations_for_run(&pool, &run.id).await.unwrap(),
            vec![operation]
        );
    }

    #[tokio::test]
    async fn sync_events_lifecycle() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;

        let event = add_sync_event(&pool, &task.id, "notes/a.txt", "file_changed")
            .await
            .unwrap();
        assert_eq!(event.status, "pending");

        let pending = list_pending_sync_events(&pool, 10).await.unwrap();
        assert_eq!(pending.len(), 1);

        mark_sync_event_processed(&pool, &event.id).await.unwrap();
        let pending = list_pending_sync_events(&pool, 10).await.unwrap();
        assert!(pending.is_empty());
    }

    #[tokio::test]
    async fn remove_sync_task_deletes_dependent_state() {
        let (_temp_dir, pool) = test_pool().await;
        let task = test_task(&pool).await;
        let entry = file_entry(&task.id, "notes/a.txt", "hash-a", 12);
        upsert_file_index_entries(&pool, &[entry]).await.unwrap();
        add_sync_event(&pool, &task.id, "notes/a.txt", "file_changed")
            .await
            .unwrap();
        let run = create_sync_run(&pool, Some(&task.id), "manual_sync")
            .await
            .unwrap();
        add_sync_operation(
            &pool,
            &run.id,
            &CreateSyncOperation {
                relative_path: "notes/a.txt".to_owned(),
                operation_kind: "create_file".to_owned(),
                status: "success".to_owned(),
                size_bytes: Some(12),
                error_message: None,
            },
        )
        .await
        .unwrap();

        assert!(remove_sync_task(&pool, &task.id).await.unwrap());
        assert_eq!(get_sync_task(&pool, &task.id).await.unwrap(), None);
        assert!(list_file_index_for_task(&pool, &task.id)
            .await
            .unwrap()
            .is_empty());
        assert!(list_recent_sync_runs(&pool, 10).await.unwrap().is_empty());
        assert!(list_pending_sync_events(&pool, 10)
            .await
            .unwrap()
            .is_empty());
    }

    async fn test_pool() -> (tempfile::TempDir, SqlitePool) {
        let temp_dir = tempfile::tempdir().unwrap();
        let pool = initialize_database(&temp_dir.path().join("state/turbosync.db"))
            .await
            .unwrap();

        (temp_dir, pool)
    }

    async fn test_task(pool: &SqlitePool) -> SyncTask {
        let node = add_node(
            pool,
            &CreateNodeRequest {
                name: "local".to_owned(),
                endpoint: "local".to_owned(),
                public_key: None,
            },
        )
        .await
        .unwrap();

        add_sync_task(
            pool,
            &CreateTaskRequest::one_way(
                "docs".to_owned(),
                "/tmp/docs".to_owned(),
                node.id,
                "/tmp/backup".to_owned(),
            ),
        )
        .await
        .unwrap()
    }

    fn file_entry(
        task_id: &str,
        relative_path: &str,
        content_hash: &str,
        size_bytes: i64,
    ) -> FileIndexEntry {
        let now = now_text();

        FileIndexEntry {
            task_id: task_id.to_owned(),
            relative_path: relative_path.to_owned(),
            file_kind: "file".to_owned(),
            size_bytes: Some(size_bytes),
            modified_at: Some(now.clone()),
            content_hash: Some(content_hash.to_owned()),
            deleted: false,
            last_seen_at: now.clone(),
            last_synced_at: None,
            updated_at: now,
        }
    }
}
