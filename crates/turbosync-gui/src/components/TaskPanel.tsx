import type { Node, SyncRun, SyncTask } from '../types/turbosync';
import {
  formatUnixTime,
  nodeLabel,
  runSummary,
  shortId,
  statusLabel,
  taskDirectionLabel,
  triggerLabel,
} from '../lib/format';
import { EmptyState } from './EmptyState';
import { StatusBadge } from './StatusBadge';

interface TaskPanelProps {
  tasks: SyncTask[];
  nodes: Node[];
  runs: SyncRun[];
  watchedTaskIds: string[];
  selectedTaskId?: string;
  busy: boolean;
  onSelect: (taskId: string) => void;
  onSync: (taskId: string) => void;
  onStartWatch: (taskId: string) => void;
  onStopWatch: (taskId: string) => void;
  onRemove: (taskId: string) => void;
}

function runStatusTone(status: string) {
  if (status === 'success') return 'green';
  if (status === 'failed') return 'red';
  if (status === 'running') return 'blue';
  return 'slate';
}

export function TaskPanel({
  tasks,
  nodes,
  runs,
  watchedTaskIds,
  selectedTaskId,
  busy,
  onSelect,
  onSync,
  onStartWatch,
  onStopWatch,
  onRemove,
}: TaskPanelProps) {
  return (
    <section className="panel p-5">
      <div className="mb-5 flex items-center justify-between">
        <div>
          <p className="section-kicker">同步任务</p>
          <h2 className="section-title">管理同步路线</h2>
        </div>
        <span className="panel-count">{tasks.length} 个</span>
      </div>

      {tasks.length === 0 ? (
        <EmptyState title="还没有同步任务" description="选择目标节点后创建任务，飞梭会帮你同步并监听变更。" />
      ) : (
        <div className="grid gap-3">
          {tasks.map((task) => {
            const node = nodes.find((item) => item.id === task.target_node_id);
            const watching = watchedTaskIds.includes(task.id);
            const latestRun = runs.find((run) => run.task_id === task.id);
            return (
              <article
                className={`task-card ${selectedTaskId === task.id ? 'is-selected' : ''}`}
                key={task.id}
                onClick={() => onSelect(task.id)}
              >
                <div className="task-card-head">
                  <div className="min-w-0">
                    <div className="task-title-row">
                      <h3>{task.name}</h3>
                      <StatusBadge label={taskDirectionLabel(task)} tone={task.direction === 'two_way' ? 'blue' : 'slate'} />
                      <StatusBadge label={watching ? '监听中' : '未监听'} tone={watching ? 'green' : 'amber'} />
                      {latestRun && <StatusBadge label={`上次${statusLabel(latestRun.status)}`} tone={runStatusTone(latestRun.status)} />}
                    </div>
                    <p className="task-path">{task.source_path}</p>
                    <p className="task-target">→ {nodeLabel(node)}:{task.target_path}</p>
                    {latestRun && (
                      <p className="task-run">
                        {triggerLabel(latestRun.trigger_kind)} · {runSummary(latestRun)} · {formatUnixTime(latestRun.finished_at, '进行中')}
                      </p>
                    )}
                  </div>
                  <span className="task-id">{shortId(task.id)}</span>
                </div>

                <div className="task-actions">
                  <button className="chip-button chip-button-primary" disabled={busy} onClick={(event) => { event.stopPropagation(); onSync(task.id); }} type="button">同步</button>
                  {watching ? (
                    <button className="chip-button" disabled={busy} onClick={(event) => { event.stopPropagation(); onStopWatch(task.id); }} type="button">停止监听</button>
                  ) : (
                    <button className="chip-button" disabled={busy} onClick={(event) => { event.stopPropagation(); onStartWatch(task.id); }} type="button">开启监听</button>
                  )}
                  <button className="danger-chip" disabled={busy} onClick={(event) => { event.stopPropagation(); onRemove(task.id); }} type="button">删除</button>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </section>
  );
}
