import type { Node } from '../types/turbosync';
import { formatUnixTime, healthLabel, shortId } from '../lib/format';
import { EmptyState } from './EmptyState';
import { StatusBadge } from './StatusBadge';

interface NodePanelProps {
  nodes: Node[];
  selectedNodeId?: string;
  busy: boolean;
  onSelect: (nodeId: string) => void;
  onRemove: (nodeId: string) => void;
}

function healthTone(status: string) {
  if (status === 'connected') return 'green';
  if (status === 'failed') return 'red';
  return 'amber';
}

function healthDetail(node: Node) {
  if (node.health_message) return node.health_message;
  if (node.health_status === 'connected') {
    return node.last_checked_at ? `最近检查 ${formatUnixTime(node.last_checked_at)}` : '连接正常';
  }
  if (node.health_status === 'failed') return '连接失败';
  return '等待健康检查';
}

export function NodePanel({ nodes, selectedNodeId, busy, onSelect, onRemove }: NodePanelProps) {
  return (
    <section id="nodes" className="panel p-5">
      <div className="mb-5 flex items-center justify-between">
        <div>
          <p className="section-kicker">设备节点</p>
          <h2 className="section-title">同步节点</h2>
        </div>
        <span className="panel-count">{nodes.length} 个</span>
      </div>

      {nodes.length === 0 ? (
        <EmptyState title="还没有节点" description="先添加本机、NAS、服务器或 Docker Agent，随后创建同步任务。" />
      ) : (
        <div className="space-y-3">
          {nodes.map((node) => (
            <button
              className={`node-card ${selectedNodeId === node.id ? 'is-selected' : ''}`}
              key={node.id}
              onClick={() => onSelect(node.id)}
              type="button"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <h3>{node.name}</h3>
                  <p>{node.endpoint}</p>
                </div>
                <StatusBadge label={healthLabel(node.health_status)} tone={healthTone(node.health_status)} />
              </div>
              <div className="node-meta">
                <span>ID {shortId(node.id)}</span>
                <span>{healthDetail(node)}</span>
              </div>
              <div className="mt-3 flex justify-end">
                <span
                  className="danger-text-action"
                  onClick={(event) => {
                    event.stopPropagation();
                    if (!busy) onRemove(node.id);
                  }}
                >
                  删除
                </span>
              </div>
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
