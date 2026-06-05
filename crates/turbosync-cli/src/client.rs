use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Serialize};
use turbosync_core::{config, models::*};

pub struct AgentClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgentClient {
    pub fn from_config() -> Result<Self> {
        let paths = config::resolve_paths()?;
        let config = config::load_config_at(&paths)
            .with_context(|| "TurboSync is not initialized; run `tsync init` first")?;

        Ok(Self {
            base_url: format!("http://{}", config.agent_addr),
            client: reqwest::Client::new(),
        })
    }

    pub async fn status(&self) -> Result<StatusResponse> {
        self.get("/v1/status").await
    }

    pub async fn health_check(&self) -> Result<()> {
        let response = self
            .client
            .get(self.url("/health"))
            .send()
            .await
            .context("local sync service is not running")?;
        Self::expect_success(response, "local sync service health check failed").await?;
        Ok(())
    }

    pub async fn add_node(&self, request: &CreateNodeRequest) -> Result<Node> {
        self.post("/v1/nodes", request).await
    }

    pub async fn list_nodes(&self) -> Result<Vec<Node>> {
        self.get("/v1/nodes").await
    }

    pub async fn remove_node(&self, node_id: &str) -> Result<bool> {
        self.delete(&format!("/v1/nodes/{node_id}")).await
    }

    pub async fn add_task(&self, request: &CreateTaskRequest) -> Result<SyncTask> {
        self.post("/v1/tasks", request).await
    }

    pub async fn list_tasks(&self) -> Result<Vec<SyncTask>> {
        self.get("/v1/tasks").await
    }

    pub async fn remove_task(&self, task_id: &str) -> Result<bool> {
        self.delete(&format!("/v1/tasks/{task_id}")).await
    }

    pub async fn rescan_task(&self, task_id: &str) -> Result<RescanResponse> {
        self.post_empty(&format!("/v1/tasks/{task_id}/rescan"))
            .await
    }

    pub async fn sync_task(&self, task_id: &str) -> Result<SyncResponse> {
        self.post_empty(&format!("/v1/tasks/{task_id}/sync")).await
    }

    pub async fn start_watch(&self, task_id: &str) -> Result<WatchStatus> {
        self.post_empty(&format!("/v1/tasks/{task_id}/watch")).await
    }

    pub async fn stop_watch(&self, task_id: &str) -> Result<bool> {
        self.delete(&format!("/v1/tasks/{task_id}/watch")).await
    }

    pub async fn get_watch_status(&self, task_id: &str) -> Result<WatchStatus> {
        self.get(&format!("/v1/tasks/{task_id}/watch")).await
    }

    pub async fn logs(&self, limit: u16) -> Result<LogsResponse> {
        self.get(&format!("/v1/logs?limit={limit}")).await
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .get(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?;
        let response = Self::expect_success(response, "agent request failed").await?;

        response
            .json()
            .await
            .context("failed to decode agent response")
    }

    async fn post<T: Serialize, U: DeserializeOwned>(&self, path: &str, body: &T) -> Result<U> {
        let response = self
            .client
            .post(self.url(path))
            .json(body)
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?;
        let response = Self::expect_success(response, "agent request failed").await?;

        response
            .json()
            .await
            .context("failed to decode agent response")
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let response = self
            .client
            .post(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?;
        let response = Self::expect_success(response, "agent request failed").await?;

        response
            .json()
            .await
            .context("failed to decode agent response")
    }

    async fn delete(&self, path: &str) -> Result<bool> {
        let response = self
            .client
            .delete(self.url(path))
            .send()
            .await
            .context("agent is not running; start it with `tsync agent run`")?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(true),
            StatusCode::NOT_FOUND => Ok(false),
            _ => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                if body.trim().is_empty() {
                    anyhow::bail!("agent request failed: {status}");
                }
                anyhow::bail!("agent request failed: {status}: {body}");
            }
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn expect_success(
        response: reqwest::Response,
        context: &str,
    ) -> Result<reqwest::Response> {
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if body.trim().is_empty() {
            anyhow::bail!("{context}: {status}");
        }
        anyhow::bail!("{context}: {status}: {body}");
    }
}
