const terminalLines = [
  'tsync node add nas 192.168.1.20:38746',
  'tsync task add documents --source ~/Documents',
  'tsync sync documents',
];

const healthItems = [
  ['source', '~/Documents'],
  ['target', 'nas:/backup/Documents'],
  ['watch', '500ms debounce'],
  ['transport', 'QUIC ready'],
];

export function Hero(): JSX.Element {
  return (
    <section className="hero-section" id="top" aria-labelledby="hero-title">
      <div className="hero-copy">
        <p className="eyebrow">Rust-native file sync · GUI / TUI / CLI</p>
        <h1 id="hero-title">把目录同步变成一条可观察的飞行路线。</h1>
        <p className="hero-lede">
          TurboSync / 飞梭同步是一个面向开发目录、NAS、服务器和 Homelab 的跨平台文件同步工具。它用本机 Agent、清晰任务模型和 QUIC 传输，把“源目录 → 目标目录”做得简单、可控、可自托管。
        </p>
        <div className="hero-actions" aria-label="主要操作">
          <a className="button-primary" href="#quickstart">
            查看快速开始
          </a>
          <a className="button-secondary" href="https://github.com/fzf54122/turbo_sync">
            阅读 GitHub
          </a>
        </div>
        <dl className="hero-stats" aria-label="项目核心指标">
          <div>
            <dt>3</dt>
            <dd>用户入口</dd>
          </div>
          <div>
            <dt>500ms</dt>
            <dd>监听防抖</dd>
          </div>
          <div>
            <dt>QUIC</dt>
            <dd>远端传输</dd>
          </div>
        </dl>
      </div>

      <div className="hero-visual" aria-label="TurboSync 同步路线示意图">
        <div className="orb orb-one" />
        <div className="orb orb-two" />
        <div className="route-panel">
          <div className="route-panel-header">
            <span className="status-dot" />
            <span>route: documents</span>
            <span className="panel-chip">healthy</span>
          </div>

          <div className="node-map">
            <div className="sync-node source-node">
              <span>DEV</span>
              <strong>Workstation</strong>
            </div>
            <div className="route-line" aria-hidden="true">
              <span />
            </div>
            <div className="sync-node target-node">
              <span>NAS</span>
              <strong>Backup bay</strong>
            </div>
          </div>

          <div className="terminal-card" aria-label="同步命令示例">
            {terminalLines.map((line) => (
              <code key={line}>$ {line}</code>
            ))}
          </div>

          <div className="health-grid">
            {healthItems.map(([label, value]) => (
              <div key={label}>
                <span>{label}</span>
                <strong>{value}</strong>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
