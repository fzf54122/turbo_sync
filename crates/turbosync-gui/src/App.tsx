import { listen } from '@tauri-apps/api/event';
import { useEffect, useMemo, useState } from 'react';
import {
  addNode,
  addTask,
  ensureAgentReady,
  loadSnapshot,
  removeNode,
  removeTask,
  rescanTask,
  startWatch,
  stopWatch,
  syncTask,
} from './api/agentClient';
import { ActivityFeed } from './components/ActivityFeed';
import { AppShell } from './components/AppShell';
import { AddNodeForm } from './components/forms/AddNodeForm';
import { AddTaskForm } from './components/forms/AddTaskForm';
import { MetricCard } from './components/MetricCard';
import { NodePanel } from './components/NodePanel';
import { TaskPanel } from './components/TaskPanel';
import { formatUnixTime, nodeLabel, shortId, taskDirectionLabel } from './lib/format';
import type { AgentReadyResponse, CreateNodeRequest, CreateTaskRequest, Snapshot } from './types/turbosync';

type Theme = 'light' | 'dark';
type ActiveForm = 'node' | 'task' | null;

function initialTheme(): Theme {
  return window.localStorage.getItem('feisuo-theme') === 'dark' ? 'dark' : 'light';
}

export default function App() {
  const [agent, setAgent] = useState<AgentReadyResponse | undefined>();
  const [snapshot, setSnapshot] = useState<Snapshot | undefined>();
  const [selectedNodeId, setSelectedNodeId] = useState('');
  const [selectedTaskId, setSelectedTaskId] = useState('');
  const [activeForm, setActiveForm] = useState<ActiveForm>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>(initialTheme);

  const selectedNode = useMemo(
    () => snapshot?.nodes.find((node) => node.id === selectedNodeId),
    [selectedNodeId, snapshot?.nodes],
  );
  const selectedTask = useMemo(
    () => snapshot?.tasks.find((task) => task.id === selectedTaskId),
    [selectedTaskId, snapshot?.tasks],
  );
  const selectedTaskNode = useMemo(
    () => snapshot?.nodes.find((node) => node.id === selectedTask?.target_node_id),
    [selectedTask?.target_node_id, snapshot?.nodes],
  );

  useEffect(() => {
    void boot();
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem('feisuo-theme', theme);
  }, [theme]);

  useEffect(() => {
    let mounted = true;
    let unlistenRefresh: (() => void) | undefined;
    let unlistenTheme: (() => void) | undefined;

    void listen('menu://refresh', () => {
      void refresh();
    }).then((unlisten) => {
      if (mounted) unlistenRefresh = unlisten;
      else unlisten();
    });

    void listen('menu://toggle-theme', () => {
      toggleTheme();
    }).then((unlisten) => {
      if (mounted) unlistenTheme = unlisten;
      else unlisten();
    });

    return () => {
      mounted = false;
      unlistenRefresh?.();
      unlistenTheme?.();
    };
  }, []);

  useEffect(() => {
    if (!snapshot) return;

    if (snapshot.nodes.length === 0) {
      setSelectedNodeId('');
    } else if (!snapshot.nodes.some((node) => node.id === selectedNodeId)) {
      setSelectedNodeId(snapshot.nodes[0].id);
    }

    if (snapshot.tasks.length === 0) {
      setSelectedTaskId('');
    } else if (!snapshot.tasks.some((task) => task.id === selectedTaskId)) {
      setSelectedTaskId(snapshot.tasks[0].id);
    }
  }, [selectedNodeId, selectedTaskId, snapshot]);

  async function boot() {
    await withBusy(async () => {
      const ready = await ensureAgentReady();
      setAgent(ready);
      setSnapshot(await loadSnapshot());
    });
  }

  async function refresh() {
    await withBusy(async () => setSnapshot(await loadSnapshot()));
  }

  async function mutate(action: () => Promise<Snapshot>) {
    await withBusy(async () => setSnapshot(await action()));
  }

  function toggleTheme() {
    setTheme((current) => (current === 'light' ? 'dark' : 'light'));
  }

  async function submitNode(request: CreateNodeRequest) {
    await mutate(() => addNode(request));
    setActiveForm(null);
  }

  async function submitTask(request: CreateTaskRequest) {
    await mutate(() => addTask(request));
    setActiveForm(null);
  }

  async function withBusy(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setBusy(false);
    }
  }

  const connected = snapshot?.nodes.filter((node) => node.health_status === 'connected').length ?? 0;
  const failed = snapshot?.nodes.filter((node) => node.health_status === 'failed').length ?? 0;
  const watching = snapshot?.watched_task_ids.length ?? 0;
  const hasNodes = (snapshot?.nodes.length ?? 0) > 0;
  const hasTasks = (snapshot?.tasks.length ?? 0) > 0;

  return (
    <AppShell
      agent={agent}
      busy={busy}
      error={error}
      onRefresh={refresh}
      onToggleTheme={toggleTheme}
      status={snapshot?.status}
      theme={theme}
    >
      <div className="home-layout">
        <section className="hero-grid">
          <div className="hero-panel">
            <div>
              <p className="section-kicker">飞梭同步是做什么的</p>
              <h1 className="hero-title">把一个文件夹，自动同步到另一台设备或目录</h1>
              <p className="hero-copy">
                先添加设备，再创建同步路线。之后可以手动同步，也可以开启监听，文件变化后自动处理。
              </p>
            </div>
            <div className="hero-actions">
              <button className="primary-button px-5" onClick={() => setActiveForm('task')} type="button">创建同步路线</button>
              <button className="secondary-button px-4" onClick={() => setActiveForm('node')} type="button">添加设备</button>
            </div>
          </div>

          <section className="route-panel route-panel-featured">
            <div>
              <p className="section-kicker">当前正在同步</p>
              <h2 className="section-title">{selectedTask ? selectedTask.name : '还没有选择同步路线'}</h2>
            </div>
            {selectedTask ? (
              <div className="route-map">
                <div className="route-endpoint">
                  <span>源目录</span>
                  <strong>{selectedTask.source_path}</strong>
                </div>
                <div className="route-arrow">→</div>
                <div className="route-endpoint">
                  <span>目标位置</span>
                  <strong>{nodeLabel(selectedTaskNode)}:{selectedTask.target_path}</strong>
                </div>
                <div className="route-meta">
                  {taskDirectionLabel(selectedTask)} · {selectedTask.conflict_mode} · ID {shortId(selectedTask.id)}
                </div>
              </div>
            ) : (
              <div className="route-empty">
                <strong>先创建一条同步路线</strong>
                <p>例如：把本机项目目录同步到 NAS，或同步到另一台 Linux 服务器。</p>
                <button className="primary-button px-5" onClick={() => setActiveForm('task')} type="button">创建同步路线</button>
              </div>
            )}
          </section>
        </section>

        <section className="guide-grid">
          <div className={`guide-card ${hasNodes ? 'is-done' : ''}`}>
            <span>1</span>
            <strong>添加设备</strong>
            <p>本机、NAS、服务器或 Docker Agent 都可以作为同步节点。</p>
          </div>
          <div className={`guide-card ${hasTasks ? 'is-done' : ''}`}>
            <span>2</span>
            <strong>创建同步路线</strong>
            <p>选择源目录、目标设备和目标目录，决定单向或双向同步。</p>
          </div>
          <div className={`guide-card ${watching > 0 ? 'is-done' : ''}`}>
            <span>3</span>
            <strong>同步或监听</strong>
            <p>点击同步立即执行；开启监听后，文件变化会自动处理。</p>
          </div>
        </section>

        <section className="metric-grid">
          <MetricCard label="同步路线" value={`${snapshot?.status.enabled_tasks ?? 0}`} detail="已创建" />
          <MetricCard label="设备节点" value={`${snapshot?.status.enabled_nodes ?? 0}`} detail={`${connected} 个可连接`} accent="from-emerald-300 to-cyan-400" />
          <MetricCard label="需要处理" value={`${failed}`} detail="异常节点" accent="from-rose-300 to-orange-400" />
          <MetricCard label="自动监听" value={`${watching}`} detail={`最近 ${formatUnixTime(snapshot?.status.last_sync_at)}`} accent="from-violet-300 to-blue-500" />
        </section>

        {activeForm && (
          <section className="panel p-5">
            <div className="mb-4 flex items-center justify-between gap-3">
              <div>
                <p className="section-kicker">{activeForm === 'node' ? '添加设备' : '创建路线'}</p>
                <h2 className="section-title">{activeForm === 'node' ? '这台设备要同步到哪里？' : '文件从哪里同步到哪里？'}</h2>
              </div>
              <button className="chip-button" onClick={() => setActiveForm(null)} type="button">收起</button>
            </div>
            {activeForm === 'node' ? (
              <AddNodeForm busy={busy} onSubmit={submitNode} />
            ) : (
              <AddTaskForm busy={busy} nodes={snapshot?.nodes ?? []} onSubmit={submitTask} selectedNodeId={selectedNodeId} />
            )}
          </section>
        )}

        <section className="workspace-grid">
          <TaskPanel
            busy={busy}
            nodes={snapshot?.nodes ?? []}
            onRemove={(taskId) => void mutate(() => removeTask(taskId))}
            onRescan={(taskId) => void mutate(() => rescanTask(taskId))}
            onSelect={setSelectedTaskId}
            onStartWatch={(taskId) => void mutate(() => startWatch(taskId))}
            onStopWatch={(taskId) => void mutate(() => stopWatch(taskId))}
            onSync={(taskId) => void mutate(() => syncTask(taskId))}
            selectedTaskId={selectedTaskId}
            tasks={snapshot?.tasks ?? []}
            watchedTaskIds={snapshot?.watched_task_ids ?? []}
          />
          <aside className="support-column">
            <NodePanel
              busy={busy}
              nodes={snapshot?.nodes ?? []}
              onRemove={(nodeId) => void mutate(() => removeNode(nodeId))}
              onSelect={setSelectedNodeId}
              selectedNodeId={selectedNodeId}
            />
            <ActivityFeed runs={snapshot?.logs.runs ?? []} />
          </aside>
        </section>
      </div>
    </AppShell>
  );
}
