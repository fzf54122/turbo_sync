import { invoke } from '@tauri-apps/api/tauri';
import type {
  AgentReadyResponse,
  CreateNodeRequest,
  CreateTaskRequest,
  FileListResponse,
  Snapshot,
  UpdateTaskRequest,
} from '../types/turbosync';

export function ensureAgentReady(): Promise<AgentReadyResponse> {
  return invoke('ensure_agent_ready');
}

export function loadSnapshot(): Promise<Snapshot> {
  return invoke('load_snapshot');
}

export function addNode(request: CreateNodeRequest): Promise<Snapshot> {
  return invoke('add_node', { request });
}

export function removeNode(nodeId: string): Promise<Snapshot> {
  return invoke('remove_node', { nodeId });
}

export function addTask(request: CreateTaskRequest): Promise<Snapshot> {
  return invoke('add_task', { request });
}

export function updateTask(taskId: string, updates: UpdateTaskRequest): Promise<Snapshot> {
  return invoke('update_task', { taskId, updates });
}

export function removeTask(taskId: string): Promise<Snapshot> {
  return invoke('remove_task', { taskId });
}

export function syncTask(taskId: string): Promise<Snapshot> {
  return invoke('sync_task', { taskId });
}

export function rescanTask(taskId: string): Promise<Snapshot> {
  return invoke('rescan_task', { taskId });
}

export function startWatch(taskId: string): Promise<Snapshot> {
  return invoke('start_watch', { taskId });
}

export function stopWatch(taskId: string): Promise<Snapshot> {
  return invoke('stop_watch', { taskId });
}

export function listTaskFiles(taskId: string): Promise<FileListResponse> {
  return invoke('list_task_files', { taskId });
}
