use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub node_id: String,
    pub enabled_nodes: i64,
    pub enabled_tasks: i64,
    pub pending_events: i64,
    pub last_sync_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub public_key: Option<String>,
    pub enabled: bool,
    #[serde(default = "default_node_health_status")]
    pub health_status: String,
    pub health_message: Option<String>,
    pub last_checked_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

fn default_node_health_status() -> String {
    "unchecked".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub endpoint: String,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SyncTask {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub target_node_id: String,
    pub target_path: String,
    pub direction: String,
    pub delete_mode: String,
    #[serde(default = "default_conflict_mode")]
    pub conflict_mode: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

fn default_conflict_mode() -> String {
    "newest_wins".to_owned()
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct FileIndexEntry {
    pub task_id: String,
    pub relative_path: String,
    pub file_kind: String,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub content_hash: Option<String>,
    pub deleted: bool,
    pub last_seen_at: String,
    pub last_synced_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SyncRun {
    pub id: String,
    pub task_id: Option<String>,
    pub trigger_kind: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub files_scanned: i64,
    pub files_changed: i64,
    pub files_failed: i64,
    pub bytes_sent: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SyncOperation {
    pub id: String,
    pub sync_run_id: String,
    pub relative_path: String,
    pub operation_kind: String,
    pub status: String,
    pub size_bytes: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RescanResponse {
    pub run: SyncRun,
    pub files_scanned: i64,
    pub files_changed: i64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SyncResponse {
    pub run: SyncRun,
    pub operations: Vec<SyncOperation>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct LogsResponse {
    pub runs: Vec<SyncRun>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub source_path: String,
    pub target_node_id: String,
    pub target_path: String,
    pub direction: String,
    pub delete_mode: String,
    #[serde(default = "default_conflict_mode")]
    pub conflict_mode: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SyncEvent {
    pub id: String,
    pub task_id: String,
    pub relative_path: String,
    pub event_kind: String,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
    pub processed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct WatchStatus {
    pub task_id: String,
    pub watching: bool,
    pub source_path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct FileListResponse {
    pub task_id: String,
    pub files: Vec<FileIndexEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SseEvent {
    pub event: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SyncProgressEvent {
    pub task_id: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
}

impl CreateTaskRequest {
    #[must_use]
    pub fn one_way(
        name: String,
        source_path: String,
        target_node_id: String,
        target_path: String,
    ) -> Self {
        Self {
            name,
            source_path,
            target_node_id,
            target_path,
            direction: "one_way".to_owned(),
            delete_mode: "propagate".to_owned(),
            conflict_mode: "newest_wins".to_owned(),
        }
    }

    #[must_use]
    pub fn two_way(
        name: String,
        source_path: String,
        target_node_id: String,
        target_path: String,
        conflict_mode: String,
    ) -> Self {
        Self {
            name,
            source_path,
            target_node_id,
            target_path,
            direction: "two_way".to_owned(),
            delete_mode: "propagate".to_owned(),
            conflict_mode,
        }
    }
}
