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
    pub created_at: String,
    pub updated_at: String,
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
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub source_path: String,
    pub target_node_id: String,
    pub target_path: String,
    pub direction: String,
    pub delete_mode: String,
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
        }
    }
}
