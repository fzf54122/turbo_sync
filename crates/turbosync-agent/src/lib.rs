use std::{collections::BTreeSet, net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;
use turbosync_core::{config, models::*};
use turbosync_storage::{CreateSyncOperation, FinishSyncRun};
use turbosync_sync::{apply_local_operations, plan_changes, scan_source, ScannedEntry};

#[derive(Clone)]
struct AppState {
    node_id: String,
    pool: sqlx::SqlitePool,
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    limit: Option<i64>,
}

pub async fn run_foreground() -> Result<()> {
    let config = config::init_config()?;
    let pool = turbosync_storage::initialize_database(&config.db_path).await?;
    let addr: SocketAddr = config
        .agent_addr
        .parse()
        .with_context(|| format!("invalid agent address {}", config.agent_addr))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind local agent API on {addr}"))?;

    println!("TurboSync agent listening on http://{addr}");
    tracing::info!(%addr, "TurboSync agent started");

    axum::serve(listener, router(config.node_id, pool))
        .await
        .context("local agent API stopped unexpectedly")
}

fn router(node_id: String, pool: sqlx::SqlitePool) -> Router {
    let state = AppState { node_id, pool };

    Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/nodes", get(list_nodes).post(add_node))
        .route("/v1/nodes/:node_id", delete(remove_node))
        .route("/v1/tasks", get(list_tasks).post(add_task))
        .route("/v1/tasks/:task_id", delete(remove_task))
        .route(
            "/v1/tasks/:task_id/rescan",
            axum::routing::post(rescan_task),
        )
        .route("/v1/tasks/:task_id/sync", axum::routing::post(sync_task))
        .route("/v1/logs", get(logs))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        version: turbosync_core::VERSION.to_owned(),
    })
}

async fn status(State(state): State<AppState>) -> ApiResult<StatusResponse> {
    turbosync_storage::agent_status(&state.pool, &state.node_id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn add_node(
    State(state): State<AppState>,
    Json(request): Json<CreateNodeRequest>,
) -> ApiResult<Node> {
    turbosync_storage::add_node(&state.pool, &request)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn list_nodes(State(state): State<AppState>) -> ApiResult<Vec<Node>> {
    turbosync_storage::list_nodes(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn remove_node(State(state): State<AppState>, Path(node_id): Path<String>) -> Response {
    match turbosync_storage::remove_node(&state.pool, &node_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => ApiError::from(error).into_response(),
    }
}

async fn add_task(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> ApiResult<SyncTask> {
    turbosync_storage::add_sync_task(&state.pool, &request)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn list_tasks(State(state): State<AppState>) -> ApiResult<Vec<SyncTask>> {
    turbosync_storage::list_sync_tasks(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn remove_task(State(state): State<AppState>, Path(task_id): Path<String>) -> Response {
    match turbosync_storage::remove_sync_task(&state.pool, &task_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => ApiError::from(error).into_response(),
    }
}

async fn rescan_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<RescanResponse> {
    let task = load_enabled_task(&state.pool, &task_id).await?;
    let run = turbosync_storage::create_sync_run(&state.pool, Some(&task.id), "manual_rescan")
        .await
        .map_err(ApiError::from)?;

    match execute_rescan(&state.pool, &task, &run).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            finish_failed_run(&state.pool, &run.id, &error).await;
            Err(ApiError::from(error))
        }
    }
}

async fn sync_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<SyncResponse> {
    let task = load_enabled_task(&state.pool, &task_id).await?;
    let run = turbosync_storage::create_sync_run(&state.pool, Some(&task.id), "manual_sync")
        .await
        .map_err(ApiError::from)?;

    match execute_sync(&state.pool, &task, &run).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => {
            finish_failed_run(&state.pool, &run.id, &error).await;
            Err(ApiError::from(error))
        }
    }
}

async fn logs(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> ApiResult<LogsResponse> {
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    let runs = turbosync_storage::list_recent_sync_runs(&state.pool, limit)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(LogsResponse { runs }))
}

async fn execute_rescan(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
) -> Result<RescanResponse> {
    let previous = turbosync_storage::list_file_index_for_task(pool, &task.id).await?;
    let scan = scan_source(&PathBuf::from(&task.source_path))?;
    let operations = plan_changes(&previous, &scan.entries);
    let missing_paths = missing_tracked_paths(&previous, &scan.entries);
    let now = turbosync_storage::now_text();
    let entries = scanned_entries_to_index(&task.id, &scan.entries, &now);

    turbosync_storage::upsert_file_index_entries(pool, &entries).await?;
    turbosync_storage::mark_file_index_deleted(pool, &task.id, &missing_paths).await?;
    let finished_run = turbosync_storage::finish_sync_run(
        pool,
        &run.id,
        &FinishSyncRun {
            status: "success".to_owned(),
            files_scanned: scan.files_scanned,
            files_changed: usize_to_i64(operations.len()),
            files_failed: 0,
            bytes_sent: 0,
            error_message: None,
        },
    )
    .await?;

    Ok(RescanResponse {
        run: finished_run,
        files_scanned: scan.files_scanned,
        files_changed: usize_to_i64(operations.len()),
    })
}

async fn execute_sync(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
) -> Result<SyncResponse> {
    let previous = turbosync_storage::list_file_index_for_task(pool, &task.id).await?;
    let scan = scan_source(&PathBuf::from(&task.source_path))?;
    let operations = plan_changes(&previous, &scan.entries);
    let apply_summary = apply_local_operations(
        &PathBuf::from(&task.source_path),
        &PathBuf::from(&task.target_path),
        &operations,
    )?;
    let now = turbosync_storage::now_text();
    let entries = scanned_entries_to_index(&task.id, &scan.entries, &now);
    let missing_paths = missing_tracked_paths(&previous, &scan.entries);
    let synced_paths: Vec<String> = apply_summary
        .succeeded
        .iter()
        .map(|operation| operation.relative_path.clone())
        .collect();

    turbosync_storage::upsert_file_index_entries(pool, &entries).await?;
    turbosync_storage::mark_file_index_deleted(pool, &task.id, &missing_paths).await?;
    turbosync_storage::set_file_index_synced(pool, &task.id, &synced_paths, &now).await?;

    for operation in &apply_summary.succeeded {
        turbosync_storage::add_sync_operation(
            pool,
            &run.id,
            &CreateSyncOperation {
                relative_path: operation.relative_path.clone(),
                operation_kind: operation.kind.as_str().to_owned(),
                status: "success".to_owned(),
                size_bytes: operation.size_bytes,
                error_message: None,
            },
        )
        .await?;
    }

    for operation in &apply_summary.failed {
        turbosync_storage::add_sync_operation(
            pool,
            &run.id,
            &CreateSyncOperation {
                relative_path: operation.relative_path.clone(),
                operation_kind: operation.kind.as_str().to_owned(),
                status: "failed".to_owned(),
                size_bytes: operation.size_bytes,
                error_message: Some(operation.error_message.clone()),
            },
        )
        .await?;
    }

    let status = if apply_summary.failed.is_empty() {
        "success"
    } else {
        "failed"
    };
    let finished_run = turbosync_storage::finish_sync_run(
        pool,
        &run.id,
        &FinishSyncRun {
            status: status.to_owned(),
            files_scanned: scan.files_scanned,
            files_changed: usize_to_i64(operations.len()),
            files_failed: usize_to_i64(apply_summary.failed.len()),
            bytes_sent: apply_summary.bytes_copied,
            error_message: None,
        },
    )
    .await?;
    let recorded_operations =
        turbosync_storage::list_sync_operations_for_run(pool, &run.id).await?;

    Ok(SyncResponse {
        run: finished_run,
        operations: recorded_operations,
    })
}

async fn load_enabled_task(
    pool: &sqlx::SqlitePool,
    task_id: &str,
) -> std::result::Result<SyncTask, ApiError> {
    let task = turbosync_storage::get_sync_task(pool, task_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound("sync task not found"))?;

    if !task.enabled {
        return Err(ApiError::BadRequest("sync task is disabled"));
    }

    Ok(task)
}

async fn finish_failed_run(pool: &sqlx::SqlitePool, run_id: &str, error: &anyhow::Error) {
    if let Err(finish_error) = turbosync_storage::finish_sync_run(
        pool,
        run_id,
        &FinishSyncRun {
            status: "failed".to_owned(),
            files_scanned: 0,
            files_changed: 0,
            files_failed: 1,
            bytes_sent: 0,
            error_message: Some(error.to_string()),
        },
    )
    .await
    {
        tracing::error!(error = %finish_error, "failed to mark sync run failed");
    }
}

fn scanned_entries_to_index(
    task_id: &str,
    entries: &[ScannedEntry],
    now: &str,
) -> Vec<FileIndexEntry> {
    entries
        .iter()
        .map(|entry| FileIndexEntry {
            task_id: task_id.to_owned(),
            relative_path: entry.relative_path.clone(),
            file_kind: entry.file_kind.clone(),
            size_bytes: entry.size_bytes,
            modified_at: entry.modified_at.clone(),
            content_hash: entry.content_hash.clone(),
            deleted: false,
            last_seen_at: now.to_owned(),
            last_synced_at: None,
            updated_at: now.to_owned(),
        })
        .collect()
}

fn missing_tracked_paths(previous: &[FileIndexEntry], current: &[ScannedEntry]) -> Vec<String> {
    let current_paths: BTreeSet<&str> = current
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();

    previous
        .iter()
        .filter(|entry| !entry.deleted)
        .filter(|entry| !current_paths.contains(entry.relative_path.as_str()))
        .map(|entry| entry.relative_path.clone())
        .collect()
}

fn usize_to_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

type ApiResult<T> = std::result::Result<Json<T>, ApiError>;

enum ApiError {
    NotFound(&'static str),
    BadRequest(&'static str),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Self::Internal(error) => {
                tracing::error!(error = %error, "local agent API request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "storage operation failed",
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_response_reports_version() {
        let Json(response) = health().await;

        assert_eq!(response.status, "ok");
        assert_eq!(response.version, turbosync_core::VERSION);
    }
}
