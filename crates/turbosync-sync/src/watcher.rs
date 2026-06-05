use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher as _};
use tokio::sync::{mpsc, Mutex};

const DEBOUNCE_MS: u64 = 500;

struct TaskWatch {
    source_path: PathBuf,
    deadline: Option<tokio::time::Instant>,
}

/// Handle for the file-system watcher. Cloning returns a handle that shares
/// the same underlying watcher and task set.
#[derive(Clone)]
pub struct WatcherHandle {
    tasks: Arc<Mutex<HashMap<String, TaskWatch>>>,
    watcher: Arc<Mutex<notify::RecommendedWatcher>>,
    trigger_tx: mpsc::Sender<String>,
}

impl WatcherHandle {
    /// Spawn the watcher and its background tasks.
    ///
    /// `trigger_tx` receives a `task_id` whenever a watched directory sees
    /// changes that have settled (debounced).
    pub fn spawn(trigger_tx: mpsc::Sender<String>) -> Result<Self> {
        let tasks = Arc::new(Mutex::new(HashMap::<String, TaskWatch>::new()));
        let (event_tx, mut event_rx) = mpsc::channel::<Event>(512);

        let watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            if let Ok(event) = event {
                let _ = event_tx.blocking_send(event);
            }
        })
        .context("failed to create file watcher")?;

        let watcher = Arc::new(Mutex::new(watcher));

        // Process raw notify events — update per-task debounce deadlines.
        let tasks_for_events = tasks.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if !is_fs_event(&event.kind) {
                    continue;
                }
                let deadline = tokio::time::Instant::now() + Duration::from_millis(DEBOUNCE_MS);
                let mut tasks = tasks_for_events.lock().await;
                for ts in tasks.values_mut() {
                    if event.paths.iter().any(|p| p.starts_with(&ts.source_path)) {
                        ts.deadline = Some(deadline);
                    }
                }
            }
        });

        // Ticker — fires when a task's deadline has expired.
        let tasks_for_ticker = tasks.clone();
        let tx = trigger_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(DEBOUNCE_MS / 2));
            loop {
                interval.tick().await;
                let now = tokio::time::Instant::now();
                let mut tasks = tasks_for_ticker.lock().await;
                for (task_id, ts) in tasks.iter_mut() {
                    if let Some(deadline) = ts.deadline {
                        if now >= deadline {
                            ts.deadline = None;
                            let _ = tx.send(task_id.clone()).await;
                        }
                    }
                }
            }
        });

        Ok(Self {
            tasks,
            watcher,
            trigger_tx,
        })
    }

    /// Start watching a task's source directory.
    pub async fn add_task(&self, task_id: &str, source_path: &Path) -> Result<()> {
        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(
                task_id.to_owned(),
                TaskWatch {
                    source_path: source_path.to_path_buf(),
                    deadline: None,
                },
            );
        }
        self.watcher
            .lock()
            .await
            .watch(source_path, RecursiveMode::Recursive)
            .with_context(|| format!("failed to watch {}", source_path.display()))?;
        Ok(())
    }

    /// Stop watching a task's source directory.
    pub async fn remove_task(&self, task_id: &str) -> Result<()> {
        let source_path = {
            let mut tasks = self.tasks.lock().await;
            tasks.remove(task_id).map(|ts| ts.source_path)
        };
        if let Some(path) = source_path {
            self.watcher
                .lock()
                .await
                .unwatch(&path)
                .with_context(|| format!("failed to unwatch {}", path.display()))?;
        }
        Ok(())
    }

    /// Return a sender that can be used to check which tasks are active.
    pub fn sender(&self) -> mpsc::Sender<String> {
        self.trigger_tx.clone()
    }
}

fn is_fs_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::time::timeout;

    #[tokio::test]
    async fn watcher_fires_after_file_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();

        let (tx, mut rx) = mpsc::channel::<String>(8);
        let handle = WatcherHandle::spawn(tx).unwrap();
        handle.add_task("task-1", &source).await.unwrap();

        // Create a file — should trigger after debounce.
        fs::write(source.join("hello.txt"), b"data").unwrap();

        let task_id = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for watcher event")
            .expect("channel closed unexpectedly");

        assert_eq!(task_id, "task-1");
    }

    #[tokio::test]
    async fn watcher_debounces_burst_of_events() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();

        let (tx, mut rx) = mpsc::channel::<String>(8);
        let handle = WatcherHandle::spawn(tx).unwrap();
        handle.add_task("task-1", &source).await.unwrap();

        // Rapid-fire writes should collapse into a single trigger.
        fs::write(source.join("a.txt"), b"a").unwrap();
        fs::write(source.join("b.txt"), b"b").unwrap();
        fs::write(source.join("c.txt"), b"c").unwrap();

        let first = timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out")
            .expect("closed");
        assert_eq!(first, "task-1");

        // No second event should arrive within the next second.
        let second = timeout(Duration::from_secs(1), rx.recv()).await;
        assert!(second.is_err(), "unexpected second trigger: {second:?}");
    }

    #[tokio::test]
    async fn remove_task_stops_watching() {
        let temp_dir = tempfile::tempdir().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();

        let (tx, mut rx) = mpsc::channel::<String>(8);
        let handle = WatcherHandle::spawn(tx).unwrap();
        handle.add_task("task-1", &source).await.unwrap();
        handle.remove_task("task-1").await.unwrap();

        fs::write(source.join("ignored.txt"), b"x").unwrap();

        let result = timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(result.is_err(), "received event after remove: {result:?}");
    }
}
