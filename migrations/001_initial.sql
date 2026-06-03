CREATE TABLE nodes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    public_key TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE sync_tasks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    source_path TEXT NOT NULL,
    target_node_id TEXT NOT NULL,
    target_path TEXT NOT NULL,
    direction TEXT NOT NULL DEFAULT 'one_way',
    delete_mode TEXT NOT NULL DEFAULT 'propagate',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (target_node_id) REFERENCES nodes(id)
);

CREATE TABLE file_index (
    task_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    file_kind TEXT NOT NULL,
    size_bytes INTEGER,
    modified_at TEXT,
    content_hash TEXT,
    deleted INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT NOT NULL,
    last_synced_at TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, relative_path),
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id)
);

CREATE TABLE sync_events (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    created_at TEXT NOT NULL,
    processed_at TEXT,
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id)
);

CREATE TABLE sync_runs (
    id TEXT PRIMARY KEY,
    task_id TEXT,
    trigger_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    files_scanned INTEGER NOT NULL DEFAULT 0,
    files_changed INTEGER NOT NULL DEFAULT 0,
    files_failed INTEGER NOT NULL DEFAULT 0,
    bytes_sent INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    FOREIGN KEY (task_id) REFERENCES sync_tasks(id)
);

CREATE TABLE sync_operations (
    id TEXT PRIMARY KEY,
    sync_run_id TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    operation_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    size_bytes INTEGER,
    error_message TEXT,
    created_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (sync_run_id) REFERENCES sync_runs(id)
);

CREATE INDEX idx_sync_tasks_enabled ON sync_tasks(enabled);
CREATE INDEX idx_file_index_task_deleted ON file_index(task_id, deleted);
CREATE INDEX idx_sync_events_status ON sync_events(status);
CREATE INDEX idx_sync_runs_started_at ON sync_runs(started_at);
