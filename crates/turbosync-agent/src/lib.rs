use std::{collections::HashSet, convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, sse::KeepAlive, sse::Sse, Html, IntoResponse, Response},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use turbosync_core::{config, models::*};
use turbosync_storage::{CreateSyncOperation, FinishSyncRun};
use turbosync_sync::{
    apply_local_operations, detect_conflicts, plan_changes, plan_remote_changes,
    resolve_newest_wins, scan_source, watcher::WatcherHandle, ScannedEntry,
};

#[derive(Clone)]
struct AppState {
    node_id: String,
    agent_addr: String,
    transport_addr: String,
    pool: sqlx::SqlitePool,
    watcher: WatcherHandle,
    watched_tasks: Arc<Mutex<HashSet<String>>>,
    event_tx: broadcast::Sender<SseEvent>,
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    limit: Option<i64>,
}

pub async fn run_foreground() -> Result<()> {
    let config = config::init_config()?;
    let pool = turbosync_storage::initialize_database(&config.db_path).await?;
    let stale_runs = turbosync_storage::mark_running_sync_runs_failed(
        &pool,
        "agent restarted before sync completed",
    )
    .await?;
    if stale_runs > 0 {
        tracing::warn!(stale_runs, "marked stale running sync runs as failed");
    }
    let addr: SocketAddr = config
        .agent_addr
        .parse()
        .with_context(|| format!("invalid agent address {}", config.agent_addr))?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind local agent API on {addr}"))?;

    let (trigger_tx, mut trigger_rx) = tokio::sync::mpsc::channel::<String>(64);
    let watcher = WatcherHandle::spawn(trigger_tx).context("failed to start file watcher")?;

    let (event_tx, _) = broadcast::channel::<SseEvent>(64);

    // Background sync loop — processes watcher triggers.
    let pool_for_loop = pool.clone();
    let node_id_for_loop = config.node_id.clone();
    let agent_addr_for_loop = config.agent_addr.clone();
    let transport_addr_for_loop = config.transport_addr.clone();
    let event_tx_for_loop = event_tx.clone();
    tokio::spawn(async move {
        while let Some(task_id) = trigger_rx.recv().await {
            let _ = event_tx_for_loop.send(SseEvent {
                event: "watch_triggered".to_owned(),
                data: serde_json::json!({"task_id": task_id}),
            });
            if let Err(error) = process_watch_trigger(
                &pool_for_loop,
                &task_id,
                &node_id_for_loop,
                &agent_addr_for_loop,
                &transport_addr_for_loop,
                &event_tx_for_loop,
            )
            .await
            {
                tracing::error!(%task_id, %error, "watch trigger failed");
            }
        }
    });

    // Start the transport server for Agent-to-Agent file transfer.
    let (cert, key) = config::load_agent_cert(&config.cert_dir)
        .context("failed to load agent TLS certificate")?;
    let pool_for_index = pool.clone();
    let file_index_handler: turbosync_transport::FileIndexHandler = Arc::new({
        let pool = pool_for_index.clone();
        move |task_id: String| -> Result<Vec<u8>> {
            let entries = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(turbosync_storage::list_file_index_for_task(&pool, &task_id))
            })?;
            serde_json::to_vec(&entries).context("failed to serialize file index")
        }
    });

    let pool_for_pull = pool.clone();
    let pull_file_handler: turbosync_transport::PullFileHandler = Arc::new(
        move |task_id: String, relative_path: String| -> Result<Vec<u8>> {
            let task = tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(turbosync_storage::get_sync_task(&pool_for_pull, &task_id))
            })?
            .ok_or_else(|| anyhow::anyhow!("task not found: {task_id}"))?;
            let full_path = safe_join_task_path(&PathBuf::from(&task.source_path), &relative_path)?;
            std::fs::read(&full_path)
                .with_context(|| format!("failed to read {}", full_path.display()))
        },
    );

    let transport_server =
        turbosync_transport::TransportServer::bind(&config.transport_addr, cert, key)
            .await
            .context("failed to start transport server")?
            .with_file_index_handler(file_index_handler)
            .with_pull_file_handler(pull_file_handler);
    tokio::spawn(async move {
        if let Err(error) = transport_server.run().await {
            tracing::error!(%error, "transport server stopped");
        }
    });

    let pool_for_health = pool.clone();
    let agent_addr_for_health = config.agent_addr.clone();
    let transport_addr_for_health = config.transport_addr.clone();
    tokio::spawn(async move {
        refresh_node_health(
            &pool_for_health,
            &agent_addr_for_health,
            &transport_addr_for_health,
        )
        .await;
    });

    println!("TurboSync agent listening on http://{addr}");
    tracing::info!(%addr, transport = %config.transport_addr, "TurboSync agent started");

    let watched_tasks = Arc::new(Mutex::new(HashSet::new()));
    let state = AppState {
        node_id: config.node_id,
        agent_addr: config.agent_addr,
        transport_addr: config.transport_addr,
        pool,
        watcher,
        watched_tasks,
        event_tx,
    };

    axum::serve(listener, router(state))
        .await
        .context("local agent API stopped unexpectedly")
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/gui", get(gui))
        .route("/v1/events", get(sse_events))
        .route("/v1/status", get(status))
        .route("/v1/nodes", get(list_nodes).post(add_node))
        .route("/v1/nodes/:node_id", delete(remove_node))
        .route("/v1/tasks", get(list_tasks).post(add_task))
        .route(
            "/v1/tasks/:task_id",
            axum::routing::put(update_task).delete(remove_task),
        )
        .route(
            "/v1/tasks/:task_id/rescan",
            axum::routing::post(rescan_task),
        )
        .route("/v1/tasks/:task_id/sync", axum::routing::post(sync_task))
        .route("/v1/tasks/:task_id/files", get(list_task_files))
        .route(
            "/v1/tasks/:task_id/watch",
            get(get_watch).post(start_watch).delete(stop_watch),
        )
        .route("/v1/logs", get(logs))
        .with_state(state)
}

async fn gui() -> Html<&'static str> {
    Html(include_str!("gui.html"))
}

// ── health ────────────────────────────────────────────────────────────

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_owned(),
        version: turbosync_core::VERSION.to_owned(),
    })
}

// ── status ────────────────────────────────────────────────────────────

async fn status(State(state): State<AppState>) -> ApiResult<StatusResponse> {
    turbosync_storage::agent_status(&state.pool, &state.node_id)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

// ── nodes ─────────────────────────────────────────────────────────────

async fn add_node(
    State(state): State<AppState>,
    Json(request): Json<CreateNodeRequest>,
) -> ApiResult<Node> {
    validate_node_endpoint(&request.endpoint)?;
    if let Err(error) = probe_node_endpoint(
        &request.endpoint,
        request.public_key.as_deref(),
        &state.transport_addr,
    )
    .await
    {
        return Err(ApiError::BadRequest(format!(
            "node endpoint is not reachable: {error}"
        )));
    }

    let node = turbosync_storage::add_node(&state.pool, &request)
        .await
        .map_err(ApiError::from)?;
    turbosync_storage::update_node_health(&state.pool, &node.id, "connected", None)
        .await
        .map_err(ApiError::from)?;
    turbosync_storage::get_node(&state.pool, &node.id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound("sync node not found after creation"))
        .map(Json)
}

async fn list_nodes(State(state): State<AppState>) -> ApiResult<Vec<Node>> {
    turbosync_storage::list_nodes(&state.pool)
        .await
        .map(Json)
        .map_err(ApiError::from)
}

async fn remove_node(State(state): State<AppState>, Path(node_id): Path<String>) -> Response {
    match turbosync_storage::count_sync_tasks_for_node(&state.pool, &node_id).await {
        Ok(count) if count > 0 => {
            return ApiError::BadRequest(
                "node is used by sync tasks; remove those tasks before removing the node"
                    .to_owned(),
            )
            .into_response();
        }
        Ok(_) => {}
        Err(error) => return ApiError::from(error).into_response(),
    }

    match turbosync_storage::remove_node(&state.pool, &node_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => ApiError::from(error).into_response(),
    }
}

// ── tasks ─────────────────────────────────────────────────────────────

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

// ── rescan / sync ─────────────────────────────────────────────────────

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

    match execute_sync(
        &state.pool,
        &task,
        &run,
        &state.node_id,
        &state.agent_addr,
        &state.transport_addr,
        &state.event_tx,
    )
    .await
    {
        Ok(response) => {
            let _ = state.event_tx.send(SseEvent {
                event: "sync_run_completed".to_owned(),
                data: serde_json::json!({
                    "task_id": task_id,
                    "run_id": response.run.id,
                    "status": response.run.status,
                }),
            });
            Ok(Json(response))
        }
        Err(error) => {
            finish_failed_run(&state.pool, &run.id, &error).await;
            let _ = state.event_tx.send(SseEvent {
                event: "sync_run_completed".to_owned(),
                data: serde_json::json!({
                    "task_id": task_id,
                    "status": "failed",
                    "error": error.to_string(),
                }),
            });
            Err(ApiError::from(error))
        }
    }
}

// ── watch ─────────────────────────────────────────────────────────────

async fn start_watch(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<WatchStatus> {
    let task = load_enabled_task(&state.pool, &task_id).await?;
    let source_path = PathBuf::from(&task.source_path);

    state
        .watcher
        .add_task(&task_id, &source_path)
        .await
        .map_err(ApiError::Internal)?;

    state.watched_tasks.lock().await.insert(task_id.clone());

    Ok(Json(WatchStatus {
        task_id,
        watching: true,
        source_path: task.source_path,
    }))
}

async fn stop_watch(State(state): State<AppState>, Path(task_id): Path<String>) -> Response {
    if let Err(e) = state.watcher.remove_task(&task_id).await {
        return ApiError::from(e).into_response();
    }
    state.watched_tasks.lock().await.remove(&task_id);
    StatusCode::NO_CONTENT.into_response()
}

async fn get_watch(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<WatchStatus> {
    let watched = state.watched_tasks.lock().await;
    if !watched.contains(&task_id) {
        return Err(ApiError::NotFound("task is not being watched"));
    }
    drop(watched);

    let task = load_enabled_task_no_watch_check(&state.pool, &task_id).await?;

    Ok(Json(WatchStatus {
        task_id,
        watching: true,
        source_path: task.source_path,
    }))
}

// ── logs ──────────────────────────────────────────────────────────────

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

// ── SSE events ──────────────────────────────────────────────────────────

async fn sse_events(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let data = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok(Event::default().event(event.event).data(data)))
        }
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ── task update ─────────────────────────────────────────────────────────

async fn update_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(updates): Json<UpdateTaskRequest>,
) -> ApiResult<SyncTask> {
    let updated = turbosync_storage::update_sync_task(&state.pool, &task_id, &updates)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound("sync task not found"))?;

    let _ = state.event_tx.send(SseEvent {
        event: "task_updated".to_owned(),
        data: serde_json::json!({"task_id": task_id}),
    });

    Ok(Json(updated))
}

// ── task files ──────────────────────────────────────────────────────────

async fn list_task_files(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> ApiResult<FileListResponse> {
    let files = turbosync_storage::list_file_index_for_task(&state.pool, &task_id)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(FileListResponse { task_id, files }))
}

// ── sync pipeline helpers ─────────────────────────────────────────────

async fn process_watch_trigger(
    pool: &sqlx::SqlitePool,
    task_id: &str,
    node_id: &str,
    agent_addr: &str,
    transport_addr: &str,
    event_tx: &broadcast::Sender<SseEvent>,
) -> Result<()> {
    let task = match turbosync_storage::get_sync_task(pool, task_id).await? {
        Some(t) if t.enabled => t,
        _ => return Ok(()),
    };

    turbosync_storage::add_sync_event(pool, &task.id, "", "file_changed").await?;

    let run = turbosync_storage::create_sync_run(pool, Some(&task.id), "watcher").await?;
    match execute_rescan(pool, &task, &run).await {
        Ok(_) => {}
        Err(error) => {
            finish_failed_run(pool, &run.id, &error).await;
            return Err(error);
        }
    }

    let run = turbosync_storage::create_sync_run(pool, Some(&task.id), "watcher").await?;
    if let Err(error) = execute_sync(
        pool,
        &task,
        &run,
        node_id,
        agent_addr,
        transport_addr,
        event_tx,
    )
    .await
    {
        finish_failed_run(pool, &run.id, &error).await;
        return Err(error);
    }

    Ok(())
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
    node_id: &str,
    agent_addr: &str,
    transport_addr: &str,
    event_tx: &broadcast::Sender<SseEvent>,
) -> Result<SyncResponse> {
    if task.direction == "two_way" {
        return execute_two_way_sync(
            pool,
            task,
            run,
            node_id,
            agent_addr,
            transport_addr,
            event_tx,
        )
        .await;
    }

    let previous = turbosync_storage::list_file_index_for_task(pool, &task.id).await?;
    let scan = scan_source(&PathBuf::from(&task.source_path))?;
    let operations = plan_changes(&previous, &scan.entries);
    let total = operations.len() as u32;

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": 0,
            "total": total,
            "message": "starting sync",
        }),
    });

    let now = turbosync_storage::now_text();
    let entries = scanned_entries_to_index(&task.id, &scan.entries, &now);
    let missing_paths = missing_tracked_paths(&previous, &scan.entries);

    let is_local_target = is_local_target(pool, task, node_id, agent_addr, transport_addr).await?;
    let (files_failed, bytes_sent, synced_paths, mut recorded_ops) = if is_local_target {
        apply_locally_and_record(pool, task, run, &operations).await?
    } else {
        apply_remotely_and_record(pool, task, run, &operations).await?
    };

    turbosync_storage::upsert_file_index_entries(pool, &entries).await?;
    turbosync_storage::mark_file_index_deleted(pool, &task.id, &missing_paths).await?;
    turbosync_storage::set_file_index_synced(pool, &task.id, &synced_paths, &now).await?;

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": total,
            "total": total,
            "message": "sync complete",
        }),
    });

    let status = if files_failed == 0 {
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
            files_failed,
            bytes_sent,
            error_message: None,
        },
    )
    .await?;
    recorded_ops.extend(turbosync_storage::list_sync_operations_for_run(pool, &run.id).await?);

    Ok(SyncResponse {
        run: finished_run,
        operations: recorded_ops,
    })
}

async fn execute_two_way_sync(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
    node_id: &str,
    agent_addr: &str,
    transport_addr: &str,
    event_tx: &broadcast::Sender<SseEvent>,
) -> Result<SyncResponse> {
    // Local two-way: sync both directions between two local paths.
    if is_local_target(pool, task, node_id, agent_addr, transport_addr).await? {
        return execute_local_two_way_sync(pool, task, run, event_tx).await;
    }

    // Remote two-way: pull from remote, push to remote, resolve conflicts.
    let previous = turbosync_storage::list_file_index_for_task(pool, &task.id).await?;
    let scan = scan_source(&PathBuf::from(&task.source_path))?;
    let local_ops = plan_changes(&previous, &scan.entries);
    let total = local_ops.len() as u32;

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": 0,
            "total": total,
            "message": "requesting remote index",
        }),
    });

    let node = turbosync_storage::get_node(pool, &task.target_node_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("target node not found: {}", task.target_node_id))?;
    let fingerprint = node.public_key.as_deref();

    let remote_index_json =
        turbosync_transport::request_file_index(&node.endpoint, &task.id, fingerprint).await?;
    let remote_index: Vec<FileIndexEntry> =
        serde_json::from_slice(&remote_index_json).context("failed to decode remote file index")?;

    let remote_ops = plan_remote_changes(&remote_index, &scan.entries);
    let conflicts = detect_conflicts(&local_ops, &remote_ops, &scan.entries, &remote_index);
    let (local_wins, remote_wins) = resolve_newest_wins(&conflicts);

    // Filter out conflict losers from respective operation sets.
    let local_push_ops: Vec<turbosync_sync::PlannedOperation> = local_ops
        .iter()
        .filter(|op| !remote_wins.contains(&op.relative_path))
        .cloned()
        .collect();
    let remote_pull_ops: Vec<turbosync_sync::PlannedOperation> = remote_ops
        .iter()
        .filter(|op| !local_wins.contains(&op.relative_path))
        .cloned()
        .collect();

    // Backup local copies that lose to remote.
    let now = turbosync_storage::now_text();
    for path in &remote_wins {
        let local_file = PathBuf::from(&task.source_path).join(path);
        if local_file.exists() {
            let backup = local_file.with_extension(format!("tsync-conflict-{now}"));
            let _ = std::fs::copy(&local_file, &backup);
        }
    }

    // Pull files from remote.
    let pull_summary = turbosync_transport::pull_files(
        &node.endpoint,
        &task.id,
        &remote_pull_ops,
        &PathBuf::from(&task.source_path),
        fingerprint,
    )
    .await?;

    // Push local files to remote.
    let push_summary = turbosync_transport::transfer_files(
        &node.endpoint,
        &PathBuf::from(&task.target_path),
        &local_push_ops,
        &PathBuf::from(&task.source_path),
        fingerprint,
    )
    .await?;

    let files_failed = (pull_summary.failed + push_summary.failed) as i64;
    let bytes_sent = push_summary.bytes_sent as i64;
    let total_ops = local_push_ops.len() + remote_pull_ops.len();

    let entries = scanned_entries_to_index(&task.id, &scan.entries, &now);
    let synced_paths: Vec<String> = local_push_ops
        .iter()
        .chain(remote_pull_ops.iter())
        .filter(|op| {
            operation_error(&push_summary.errors, &op.relative_path).is_none()
                && operation_error(&pull_summary.errors, &op.relative_path).is_none()
        })
        .map(|op| op.relative_path.clone())
        .collect();

    turbosync_storage::upsert_file_index_entries(pool, &entries).await?;
    turbosync_storage::set_file_index_synced(pool, &task.id, &synced_paths, &now).await?;

    let mut recorded_ops = Vec::new();
    for op in &local_push_ops {
        let error = operation_error(&push_summary.errors, &op.relative_path);
        recorded_ops.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: if error.is_some() { "failed" } else { "success" }.to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: error,
                },
            )
            .await?,
        );
    }
    for op in &remote_pull_ops {
        let error = operation_error(&pull_summary.errors, &op.relative_path);
        recorded_ops.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: if error.is_some() { "failed" } else { "success" }.to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: error,
                },
            )
            .await?,
        );
    }

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": total_ops,
            "total": total_ops,
            "message": "sync complete",
        }),
    });

    let status = if files_failed == 0 {
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
            files_changed: total_ops as i64,
            files_failed,
            bytes_sent,
            error_message: None,
        },
    )
    .await?;
    recorded_ops.extend(turbosync_storage::list_sync_operations_for_run(pool, &run.id).await?);

    Ok(SyncResponse {
        run: finished_run,
        operations: recorded_ops,
    })
}

async fn execute_local_two_way_sync(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
    event_tx: &broadcast::Sender<SseEvent>,
) -> Result<SyncResponse> {
    let source = PathBuf::from(&task.source_path);
    let target = PathBuf::from(&task.target_path);

    let source_scan = scan_source(&source)?;
    let target_scan = scan_source(&target)?;

    let source_to_target = plan_changes(
        &scanned_to_index_entries(&target_scan.entries),
        &source_scan.entries,
    );
    let target_to_source = plan_changes(
        &scanned_to_index_entries(&source_scan.entries),
        &target_scan.entries,
    );

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": 0,
            "total": source_to_target.len() + target_to_source.len(),
            "message": "local two-way sync",
        }),
    });

    let source_summary = apply_local_operations(&source, &target, &source_to_target)?;
    let target_summary = apply_local_operations(&target, &source, &target_to_source)?;

    let total_ops = source_to_target.len() + target_to_source.len();
    let files_failed = (source_summary.failed.len() + target_summary.failed.len()) as i64;
    let bytes_copied = source_summary.bytes_copied + target_summary.bytes_copied;

    let now = turbosync_storage::now_text();
    let entries: Vec<FileIndexEntry> = source_scan
        .entries
        .iter()
        .map(|e| FileIndexEntry {
            task_id: task.id.clone(),
            relative_path: e.relative_path.clone(),
            file_kind: e.file_kind.clone(),
            size_bytes: e.size_bytes,
            modified_at: e.modified_at.clone(),
            content_hash: e.content_hash.clone(),
            deleted: false,
            last_seen_at: now.clone(),
            last_synced_at: Some(now.clone()),
            updated_at: now.clone(),
        })
        .collect();

    turbosync_storage::upsert_file_index_entries(pool, &entries).await?;

    let mut recorded_ops = Vec::new();
    for op in &source_summary.succeeded {
        recorded_ops.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: "success".to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: None,
                },
            )
            .await?,
        );
    }
    for op in &target_summary.succeeded {
        recorded_ops.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: "success".to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: None,
                },
            )
            .await?,
        );
    }

    let _ = event_tx.send(SseEvent {
        event: "sync_progress".to_owned(),
        data: serde_json::json!({
            "task_id": task.id,
            "current": total_ops,
            "total": total_ops,
            "message": "local two-way sync complete",
        }),
    });

    let status = if files_failed == 0 {
        "success"
    } else {
        "failed"
    };
    let finished_run = turbosync_storage::finish_sync_run(
        pool,
        &run.id,
        &FinishSyncRun {
            status: status.to_owned(),
            files_scanned: source_scan.files_scanned + target_scan.files_scanned,
            files_changed: total_ops as i64,
            files_failed,
            bytes_sent: bytes_copied,
            error_message: None,
        },
    )
    .await?;
    recorded_ops.extend(turbosync_storage::list_sync_operations_for_run(pool, &run.id).await?);

    Ok(SyncResponse {
        run: finished_run,
        operations: recorded_ops,
    })
}

fn scanned_to_index_entries(entries: &[ScannedEntry]) -> Vec<FileIndexEntry> {
    entries
        .iter()
        .map(|e| FileIndexEntry {
            task_id: String::new(),
            relative_path: e.relative_path.clone(),
            file_kind: e.file_kind.clone(),
            size_bytes: e.size_bytes,
            modified_at: e.modified_at.clone(),
            content_hash: e.content_hash.clone(),
            deleted: false,
            last_seen_at: String::new(),
            last_synced_at: None,
            updated_at: String::new(),
        })
        .collect()
}

async fn apply_locally_and_record(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
    operations: &[turbosync_sync::PlannedOperation],
) -> Result<(i64, i64, Vec<String>, Vec<SyncOperation>)> {
    let summary = apply_local_operations(
        &PathBuf::from(&task.source_path),
        &PathBuf::from(&task.target_path),
        operations,
    )?;
    let synced_paths: Vec<String> = summary
        .succeeded
        .iter()
        .map(|op| op.relative_path.clone())
        .collect();
    let mut recorded = Vec::new();

    for op in &summary.succeeded {
        recorded.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: "success".to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: None,
                },
            )
            .await?,
        );
    }
    for op in &summary.failed {
        recorded.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: "failed".to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: Some(op.error_message.clone()),
                },
            )
            .await?,
        );
    }

    Ok((
        usize_to_i64(summary.failed.len()),
        summary.bytes_copied,
        synced_paths,
        recorded,
    ))
}

async fn apply_remotely_and_record(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    run: &SyncRun,
    operations: &[turbosync_sync::PlannedOperation],
) -> Result<(i64, i64, Vec<String>, Vec<SyncOperation>)> {
    let node = turbosync_storage::get_node(pool, &task.target_node_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("target node not found: {}", task.target_node_id))?;

    let fingerprint = node.public_key.as_deref();
    let summary = turbosync_transport::transfer_files(
        &node.endpoint,
        &PathBuf::from(&task.target_path),
        operations,
        &PathBuf::from(&task.source_path),
        fingerprint,
    )
    .await?;

    let mut synced: Vec<String> = Vec::new();
    let mut recorded: Vec<SyncOperation> = Vec::new();

    for op in operations {
        let error = operation_error(&summary.errors, &op.relative_path);
        let status = if error.is_some() { "failed" } else { "success" };

        if error.is_none() {
            synced.push(op.relative_path.clone());
        }
        recorded.push(
            turbosync_storage::add_sync_operation(
                pool,
                &run.id,
                &CreateSyncOperation {
                    relative_path: op.relative_path.clone(),
                    operation_kind: op.kind.as_str().to_owned(),
                    status: status.to_owned(),
                    size_bytes: op.size_bytes,
                    error_message: error,
                },
            )
            .await?,
        );
    }

    Ok((
        usize_to_i64(summary.failed),
        summary.bytes_sent as i64,
        synced,
        recorded,
    ))
}

async fn is_local_target(
    pool: &sqlx::SqlitePool,
    task: &SyncTask,
    node_id: &str,
    agent_addr: &str,
    transport_addr: &str,
) -> Result<bool> {
    if task.target_node_id == node_id {
        return Ok(true);
    }

    let Some(node) = turbosync_storage::get_node(pool, &task.target_node_id).await? else {
        return Ok(false);
    };

    Ok(endpoint_matches(&node.endpoint, agent_addr)
        || endpoint_matches(&node.endpoint, transport_addr))
}

fn endpoint_matches(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    match (left.parse::<SocketAddr>(), right.parse::<SocketAddr>()) {
        (Ok(left), Ok(right)) => {
            left.port() == right.port()
                && (left.ip() == right.ip()
                    || (left.ip().is_loopback() && right.ip().is_loopback()))
        }
        _ => false,
    }
}

async fn refresh_node_health(pool: &sqlx::SqlitePool, agent_addr: &str, transport_addr: &str) {
    let nodes = match turbosync_storage::list_nodes(pool).await {
        Ok(nodes) => nodes,
        Err(error) => {
            tracing::warn!(%error, "failed to load nodes for health check");
            return;
        }
    };

    for node in nodes {
        let result = if endpoint_matches(&node.endpoint, agent_addr)
            || endpoint_matches(&node.endpoint, transport_addr)
        {
            Ok(())
        } else {
            probe_node_endpoint(&node.endpoint, node.public_key.as_deref(), transport_addr).await
        };

        let (status, message) = match result {
            Ok(()) => ("connected", None),
            Err(error) => ("failed", Some(short_error_message(&error))),
        };
        if let Err(error) =
            turbosync_storage::update_node_health(pool, &node.id, status, message.as_deref()).await
        {
            tracing::warn!(node_id = %node.id, %error, "failed to update node health");
        }
    }
}

async fn probe_node_endpoint(
    endpoint: &str,
    fingerprint: Option<&str>,
    transport_addr: &str,
) -> Result<()> {
    if endpoint_matches(endpoint, transport_addr) {
        return Ok(());
    }

    turbosync_transport::check_connection(endpoint, fingerprint).await
}

fn short_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    const MAX_LEN: usize = 240;
    if message.len() <= MAX_LEN {
        message
    } else {
        format!("{}...", &message[..MAX_LEN])
    }
}

// ── task helpers ──────────────────────────────────────────────────────

async fn load_enabled_task(
    pool: &sqlx::SqlitePool,
    task_id: &str,
) -> std::result::Result<SyncTask, ApiError> {
    let task = load_enabled_task_no_watch_check(pool, task_id).await?;
    if !task.enabled {
        return Err(ApiError::BadRequest("sync task is disabled".to_owned()));
    }
    Ok(task)
}

async fn load_enabled_task_no_watch_check(
    pool: &sqlx::SqlitePool,
    task_id: &str,
) -> std::result::Result<SyncTask, ApiError> {
    turbosync_storage::get_sync_task(pool, task_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError::NotFound("sync task not found"))
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
    let current_paths: std::collections::BTreeSet<&str> = current
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

fn operation_error(errors: &[String], relative_path: &str) -> Option<String> {
    let prefix = format!("{relative_path}:");
    errors
        .iter()
        .find(|error| error.starts_with(&prefix))
        .cloned()
}

fn safe_join_task_path(root: &std::path::Path, relative_path: &str) -> Result<PathBuf> {
    let path = std::path::Path::new(relative_path);
    if path.is_absolute() {
        anyhow::bail!("relative path must not be absolute: {relative_path}");
    }

    let mut joined = root.to_path_buf();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => joined.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                anyhow::bail!("relative path escapes sync root: {relative_path}");
            }
        }
    }
    Ok(joined)
}

fn validate_node_endpoint(endpoint: &str) -> std::result::Result<(), ApiError> {
    let addr: SocketAddr = endpoint
        .parse()
        .map_err(|_| ApiError::BadRequest("node endpoint must be host:port".to_owned()))?;
    if addr.ip().is_unspecified() {
        return Err(ApiError::BadRequest(
            "node endpoint cannot use 0.0.0.0; use the remote machine IP address".to_owned(),
        ));
    }
    Ok(())
}

// ── API error handling ────────────────────────────────────────────────

type ApiResult<T> = std::result::Result<Json<T>, ApiError>;

enum ApiError {
    NotFound(&'static str),
    BadRequest(String),
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

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_response_reports_version() {
        let Json(response) = health().await;

        assert_eq!(response.status, "ok");
        assert_eq!(response.version, turbosync_core::VERSION);
    }

    #[tokio::test]
    async fn gui_page_contains_dashboard_mounts() {
        let Html(page) = gui().await;

        assert!(page.contains("TurboSync"));
        assert!(page.contains("statusStrip"));
        assert!(page.contains("/v1/tasks"));
        assert!(page.contains("/v1/nodes"));
    }
}
