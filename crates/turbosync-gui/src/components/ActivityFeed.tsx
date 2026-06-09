import type { SyncRun } from '../types/turbosync';
import { formatUnixTime, runSummary, statusLabel, triggerLabel } from '../lib/format';
import { EmptyState } from './EmptyState';
import { StatusBadge } from './StatusBadge';

interface ActivityFeedProps {
  runs: SyncRun[];
}

function statusTone(status: string) {
  if (status === 'success') return 'green';
  if (status === 'failed') return 'red';
  if (status === 'running') return 'blue';
  return 'slate';
}

export function ActivityFeed({ runs }: ActivityFeedProps) {
  return (
    <section id="activity" className="panel p-5">
      <div className="mb-5">
        <p className="section-kicker">最近活动</p>
        <h2 className="section-title">同步记录</h2>
      </div>

      {runs.length === 0 ? (
        <EmptyState title="暂无同步记录" description="执行同步、扫描或文件监听后，这里会展示最近的运行结果。" />
      ) : (
        <div className="space-y-3">
          {runs.map((run) => (
            <article className="activity-card" key={run.id}>
              <div className="activity-head">
                <div className="flex items-center gap-2">
                  <StatusBadge label={statusLabel(run.status)} tone={statusTone(run.status)} />
                  <span>{triggerLabel(run.trigger_kind)}</span>
                </div>
                <time>{formatUnixTime(run.finished_at, '进行中')}</time>
              </div>
              <p>{runSummary(run)}</p>
              {run.error_message && <p className="activity-error">{run.error_message}</p>}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}
