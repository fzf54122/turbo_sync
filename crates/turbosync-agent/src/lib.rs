use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get},
    Json, Router,
};
use turbosync_core::{config, models::*};

#[derive(Clone)]
struct AppState {
    node_id: String,
    pool: sqlx::SqlitePool,
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

type ApiResult<T> = Result<Json<T>, ApiError>;

struct ApiError(anyhow::Error);

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self.0, "local agent API request failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage operation failed",
        )
            .into_response()
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
