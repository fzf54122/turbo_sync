import { FormEvent, useEffect, useState } from 'react';
import { healthLabel, shortId } from '../../lib/format';
import type { CreateTaskRequest, Node } from '../../types/turbosync';
import { StatusBadge } from '../StatusBadge';

interface AddTaskFormProps {
  busy: boolean;
  nodes: Node[];
  selectedNodeId?: string;
  onSubmit: (request: CreateTaskRequest) => Promise<void>;
}

function nodeHealthTone(status: string) {
  if (status === 'connected') return 'green';
  if (status === 'failed') return 'red';
  return 'amber';
}

export function AddTaskForm({ busy, nodes, selectedNodeId, onSubmit }: AddTaskFormProps) {
  const [name, setName] = useState('');
  const [sourcePath, setSourcePath] = useState('');
  const [targetPath, setTargetPath] = useState('');
  const [targetNodeId, setTargetNodeId] = useState(selectedNodeId ?? '');
  const [twoWay, setTwoWay] = useState(false);

  useEffect(() => {
    if (selectedNodeId && !targetNodeId) {
      setTargetNodeId(selectedNodeId);
    }
  }, [selectedNodeId, targetNodeId]);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!targetNodeId) return;

    await onSubmit({
      name: name.trim(),
      source_path: sourcePath.trim(),
      target_node_id: targetNodeId,
      target_path: targetPath.trim(),
      direction: twoWay ? 'two_way' : 'one_way',
      delete_mode: 'propagate',
      conflict_mode: 'newest_wins',
    });
    setName('');
    setSourcePath('');
    setTargetPath('');
    setTwoWay(false);
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-[1fr_auto] gap-3">
        <label>
          <span className="sr-only">任务名</span>
          <input className="field w-full" placeholder="任务名" value={name} onChange={(event) => setName(event.target.value)} required />
        </label>
        <button className="secondary-button px-4" type="button" onClick={() => setTwoWay((value) => !value)}>
          {twoWay ? '双向' : '单向'}
        </button>
      </div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <label>
          <span className="sr-only">源目录</span>
          <input className="field w-full" placeholder="源目录" value={sourcePath} onChange={(event) => setSourcePath(event.target.value)} required />
        </label>
        <label>
          <span className="sr-only">目标目录</span>
          <input className="field w-full" placeholder="目标目录" value={targetPath} onChange={(event) => setTargetPath(event.target.value)} required />
        </label>
      </div>
      <fieldset className="node-picker">
        <legend>目标设备</legend>
        {nodes.length === 0 ? (
          <p className="node-picker-empty">还没有可用设备，请先添加同步节点。</p>
        ) : (
          <div className="node-picker-list">
            {nodes.map((node) => {
              const selected = node.id === targetNodeId;
              return (
                <label className={`node-picker-option ${selected ? 'is-selected' : ''}`} key={node.id}>
                  <input
                    checked={selected}
                    className="sr-only"
                    name="targetNodeId"
                    onChange={() => setTargetNodeId(node.id)}
                    required
                    type="radio"
                    value={node.id}
                  />
                  <span className="node-picker-main">
                    <strong>{node.name}</strong>
                    <span>{node.endpoint}</span>
                  </span>
                  <span className="node-picker-meta">
                    <span>ID {shortId(node.id)}</span>
                    <StatusBadge label={healthLabel(node.health_status)} tone={nodeHealthTone(node.health_status)} />
                  </span>
                </label>
              );
            })}
          </div>
        )}
      </fieldset>
      <button className="primary-button w-full" disabled={busy || !nodes.length || !name.trim() || !sourcePath.trim() || !targetPath.trim() || !targetNodeId} type="submit">
        创建同步任务
      </button>
    </form>
  );
}
