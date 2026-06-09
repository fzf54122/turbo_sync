import { FormEvent, useEffect, useState } from 'react';
import type { CreateTaskRequest, Node } from '../../types/turbosync';

interface AddTaskFormProps {
  busy: boolean;
  nodes: Node[];
  selectedNodeId?: string;
  onSubmit: (request: CreateTaskRequest) => Promise<void>;
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
        <input className="field" placeholder="任务名，例如 Documents" value={name} onChange={(event) => setName(event.target.value)} required />
        <button className="secondary-button px-4" type="button" onClick={() => setTwoWay((value) => !value)}>
          {twoWay ? '双向' : '单向'}
        </button>
      </div>
      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        <input className="field" placeholder="源目录" value={sourcePath} onChange={(event) => setSourcePath(event.target.value)} required />
        <input className="field" placeholder="目标目录" value={targetPath} onChange={(event) => setTargetPath(event.target.value)} required />
      </div>
      <select className="field" value={targetNodeId} onChange={(event) => setTargetNodeId(event.target.value)} required>
        <option value="" disabled>选择目标节点</option>
        {nodes.map((node) => (
          <option key={node.id} value={node.id}>{node.name} · {node.endpoint}</option>
        ))}
      </select>
      <button className="primary-button w-full" disabled={busy || !nodes.length || !name.trim() || !sourcePath.trim() || !targetPath.trim() || !targetNodeId} type="submit">
        创建同步任务
      </button>
    </form>
  );
}
