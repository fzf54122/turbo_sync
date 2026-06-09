import { listen } from '@tauri-apps/api/event';
import { useEffect, useMemo, useRef, useState } from 'react';
import {
  addNode,
  addTask,
  ensureAgentReady,
  loadSnapshot,
  removeNode,
  removeTask,
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

const FOCUSABLE_DIALOG_ELEMENTS = [
  'button:not([disabled])',
  '[href]',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

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
  const [showAbout, setShowAbout] = useState(false);
  const selectedTaskIdRef = useRef('');
  const formPanelRef = useRef<HTMLElement | null>(null);
  const aboutDialogRef = useRef<HTMLElement | null>(null);
  const aboutCloseButtonRef = useRef<HTMLButtonElement | null>(null);
  const lastFocusedElementRef = useRef<HTMLElement | null>(null);

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
    selectedTaskIdRef.current = selectedTaskId;
  }, [selectedTaskId]);

  useEffect(() => {
    if (!showAbout) return;

    const previousFocus = lastFocusedElementRef.current;
    const focusCloseButton = () => aboutCloseButtonRef.current?.focus();
    const trapDialogFocus = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setShowAbout(false);
        return;
      }

      if (event.key !== 'Tab') return;

      const focusableElements = Array.from(
        aboutDialogRef.current?.querySelectorAll<HTMLElement>(FOCUSABLE_DIALOG_ELEMENTS) ?? [],
      );
      const firstElement = focusableElements[0];
      const lastElement = focusableElements[focusableElements.length - 1];

      if (!firstElement || !lastElement) {
        event.preventDefault();
        return;
      }

      if (event.shiftKey && document.activeElement === firstElement) {
        event.preventDefault();
        lastElement.focus();
      } else if (!event.shiftKey && document.activeElement === lastElement) {
        event.preventDefault();
        firstElement.focus();
      }
    };

    focusCloseButton();
    document.addEventListener('keydown', trapDialogFocus);

    return () => {
      document.removeEventListener('keydown', trapDialogFocus);
      previousFocus?.focus();
      lastFocusedElementRef.current = null;
    };
  }, [showAbout]);

  useEffect(() => {
    if (!activeForm) return;

    const frameId = window.requestAnimationFrame(() => {
      const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      formPanelRef.current?.scrollIntoView({
        behavior: prefersReducedMotion ? 'auto' : 'smooth',
        block: 'start',
      });
    });

    return () => window.cancelAnimationFrame(frameId);
  }, [activeForm]);

  useEffect(() => {
    let mounted = true;
    const unlisteners: Array<() => void> = [];
    const registerMenu = (eventName: string, handler: () => void) => {
      void listen(eventName, handler).then((unlisten) => {
        if (mounted) unlisteners.push(unlisten);
        else unlisten();
      });
    };
    const runSelectedTaskAction = (action: (taskId: string) => Promise<Snapshot>) => {
      const taskId = selectedTaskIdRef.current;
      if (taskId) void mutate(() => action(taskId));
      else setActiveForm('task');
    };

    registerMenu('menu://refresh', () => {
      void refresh();
    });
    registerMenu('menu://toggle-theme', toggleTheme);
    registerMenu('menu://add-node', () => setActiveForm('node'));
    registerMenu('menu://add-task', () => setActiveForm('task'));
    registerMenu('menu://sync-selected', () => runSelectedTaskAction(syncTask));
    registerMenu('menu://watch-selected', () => runSelectedTaskAction(startWatch));
    registerMenu('menu://stop-watch-selected', () => runSelectedTaskAction(stopWatch));
    registerMenu('menu://about', openAbout);

    return () => {
      mounted = false;
      unlisteners.forEach((unlisten) => unlisten());
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

  function openAbout() {
    lastFocusedElementRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    setShowAbout(true);
  }

  function closeAbout() {
    setShowAbout(false);
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
              <p className="section-kicker">飞梭同步</p>
              <h1 className="hero-title">文件夹自动同步</h1>
              <p className="hero-copy">添加设备 → 创建路线 → 同步或监听。</p>
            </div>
            <div className="hero-actions">
              <button className="primary-button px-5" onClick={() => setActiveForm('task')} type="button">创建同步路线</button>
              <button className="secondary-button px-4" onClick={() => setActiveForm('node')} type="button">添加设备</button>
            </div>
          </div>

          <section className="route-panel route-panel-featured">
            <div>
              <p className="section-kicker">当前选中的同步路线</p>
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
                <button className="primary-button px-5" onClick={() => setActiveForm('task')} type="button">创建同步路线</button>
              </div>
            )}
          </section>
        </section>

        <section className="guide-grid" aria-label="快速流程">
          <div className={`guide-card ${hasNodes ? 'is-done' : ''}`}>
            <span>1</span>
            <strong>添加设备</strong>
          </div>
          <div className={`guide-card ${hasTasks ? 'is-done' : ''}`}>
            <span>2</span>
            <strong>创建路线</strong>
          </div>
          <div className={`guide-card ${watching > 0 ? 'is-done' : ''}`}>
            <span>3</span>
            <strong>同步 / 监听</strong>
          </div>
        </section>

        <section className="metric-grid">
          <MetricCard label="同步路线" value={`${snapshot?.status.enabled_tasks ?? 0}`} detail="已创建" />
          <MetricCard label="设备节点" value={`${snapshot?.status.enabled_nodes ?? 0}`} detail={`${connected} 个可连接`} accent="from-emerald-300 to-cyan-400" />
          <MetricCard label="需要处理" value={`${failed}`} detail="异常节点" accent="from-rose-300 to-orange-400" />
          <MetricCard label="自动监听" value={`${watching}`} detail={`最近 ${formatUnixTime(snapshot?.status.last_sync_at)}`} accent="from-violet-300 to-blue-500" />
        </section>

        {activeForm && (
          <section className="panel form-panel p-5" ref={formPanelRef}>
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
            onSelect={setSelectedTaskId}
            onStartWatch={(taskId) => void mutate(() => startWatch(taskId))}
            onStopWatch={(taskId) => void mutate(() => stopWatch(taskId))}
            onSync={(taskId) => void mutate(() => syncTask(taskId))}
            selectedTaskId={selectedTaskId}
            runs={snapshot?.logs.runs ?? []}
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

      {showAbout && (
        <div className="about-backdrop" role="presentation" onClick={closeAbout}>
          <section
            aria-labelledby="about-title"
            aria-modal="true"
            className="about-dialog"
            onClick={(event) => event.stopPropagation()}
            ref={aboutDialogRef}
            role="dialog"
          >
            <p className="section-kicker">关于</p>
            <h2 className="section-title" id="about-title">飞梭同步</h2>
            <p>文件夹自动同步工具，用于管理设备、同步路线和监听任务。</p>
            <button className="primary-button w-full" onClick={closeAbout} ref={aboutCloseButtonRef} type="button">知道了</button>
          </section>
        </div>
      )}
    </AppShell>
  );
}
