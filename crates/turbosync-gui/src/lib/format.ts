import type { Node, SyncRun, SyncTask } from '../types/turbosync';

export function shortId(id: string): string {
  return id.slice(0, 8);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  return `${value.toFixed(value >= 10 ? 1 : 2)} ${units[unitIndex]}`;
}

export function formatUnixTime(value?: string | null, fallback = '暂无'): string {
  if (!value) return fallback;
  const timestamp = Number(value);
  if (!Number.isFinite(timestamp) || timestamp <= 0) return fallback;
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(timestamp * 1000));
}

export function healthLabel(status: string): string {
  if (status === 'connected') return '已连接';
  if (status === 'failed') return '连接失败';
  return '未检测';
}

export function statusLabel(status: string): string {
  if (status === 'success') return '成功';
  if (status === 'failed') return '失败';
  if (status === 'running') return '运行中';
  return '未知';
}

export function triggerLabel(trigger: string): string {
  if (trigger === 'manual_sync') return '手动同步';
  if (trigger === 'manual_rescan') return '手动扫描';
  if (trigger === 'watcher') return '文件变化';
  return '其他';
}

export function taskDirectionLabel(task: SyncTask): string {
  return task.direction === 'two_way' ? '双向同步' : '单向同步';
}

export function nodeLabel(node?: Node): string {
  if (!node) return '未知节点';
  return `${node.name} ${shortId(node.id)}`;
}

export function runSummary(run: SyncRun): string {
  return `变更 ${run.files_changed} · 失败 ${run.files_failed} · 数据 ${formatBytes(run.bytes_sent)}`;
}
