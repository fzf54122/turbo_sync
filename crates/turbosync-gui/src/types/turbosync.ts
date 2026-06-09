export interface HealthResponse {
  status: string;
  version: string;
}

export interface StatusResponse {
  status: string;
  node_id: string;
  enabled_nodes: number;
  enabled_tasks: number;
  pending_events: number;
  last_sync_at?: string | null;
}

export interface Node {
  id: string;
  name: string;
  endpoint: string;
  public_key?: string | null;
  enabled: boolean;
  health_status: string;
  health_message?: string | null;
  last_checked_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateNodeRequest {
  name: string;
  endpoint: string;
  public_key?: string | null;
}

export interface SyncTask {
  id: string;
  name: string;
  source_path: string;
  target_node_id: string;
  target_path: string;
  direction: string;
  delete_mode: string;
  conflict_mode: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateTaskRequest {
  name: string;
  source_path: string;
  target_node_id: string;
  target_path: string;
  direction: string;
  delete_mode: string;
  conflict_mode: string;
}

export interface UpdateTaskRequest {
  name?: string | null;
  source_path?: string | null;
  target_path?: string | null;
  enabled?: boolean | null;
}

export interface SyncRun {
  id: string;
  task_id?: string | null;
  trigger_kind: string;
  status: string;
  started_at: string;
  finished_at?: string | null;
  files_scanned: number;
  files_changed: number;
  files_failed: number;
  bytes_sent: number;
  error_message?: string | null;
}

export interface LogsResponse {
  runs: SyncRun[];
}

export interface FileIndexEntry {
  task_id: string;
  relative_path: string;
  file_kind: string;
  size_bytes?: number | null;
  modified_at?: string | null;
  content_hash?: string | null;
  deleted: boolean;
  last_seen_at: string;
  last_synced_at?: string | null;
  updated_at: string;
}

export interface FileListResponse {
  task_id: string;
  files: FileIndexEntry[];
}

export interface Snapshot {
  status: StatusResponse;
  nodes: Node[];
  tasks: SyncTask[];
  logs: LogsResponse;
  watched_task_ids: string[];
}

export interface AgentReadyResponse {
  started: boolean;
  base_url: string;
}
