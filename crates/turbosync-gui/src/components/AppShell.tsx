import type { ReactNode } from 'react';
import type { AgentReadyResponse, StatusResponse } from '../types/turbosync';
import { formatUnixTime, shortId } from '../lib/format';
import { StatusBadge } from './StatusBadge';

interface AppShellProps {
  status?: StatusResponse;
  agent?: AgentReadyResponse;
  busy: boolean;
  error?: string | null;
  theme: 'light' | 'dark';
  onRefresh: () => void;
  onToggleTheme: () => void;
  children: ReactNode;
}

export function AppShell({ status, agent, busy, error, theme, onRefresh, onToggleTheme, children }: AppShellProps) {
  return (
    <main className={`app-root theme-${theme}`}>
      <div className="app-atmosphere" />
      <div className="relative flex h-screen min-w-0 flex-col overflow-hidden">
        <header className="app-header">
          <div className="app-header-inner">
            <div className="brand-cluster">
              <div className="brand-mark">
                <img alt="飞梭同步吉祥物" className="h-full w-full" src="/feisuo-mascot.svg" />
              </div>
              <div className="min-w-0">
                <h1 className="brand-title">飞梭同步</h1>
                <p className="brand-subtitle">文件同步工具 · 本机 {status ? shortId(status.node_id) : '初始化中'}</p>
              </div>
            </div>

            <div className="toolbar-cluster">
              <StatusBadge label={agent?.started ? '本机 Agent 已启动' : '等待本机 Agent'} tone={agent?.started ? 'green' : 'amber'} />
              <span className="last-sync-pill">最近同步 {formatUnixTime(status?.last_sync_at, '暂无记录')}</span>
              <button className="toolbar-button" disabled={busy} onClick={onRefresh} type="button">
                {busy ? '处理中' : '刷新'}
              </button>
              <button className="toolbar-button theme-toggle" onClick={onToggleTheme} type="button" aria-label={theme === 'light' ? '切换到深色模式' : '切换到浅色模式'}>
                <span>{theme === 'light' ? '深色' : '浅色'}</span>
              </button>
            </div>
          </div>
        </header>

        {error && (
          <div className="shrink-0 border-b border-rose-400/20 bg-rose-50/80 px-6 py-3 text-sm font-semibold text-rose-700">
            <div className="mx-auto max-w-[1520px]">{error}</div>
          </div>
        )}

        <section className="app-content">
          {children}
        </section>
      </div>
    </main>
  );
}
