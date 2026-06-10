export function SiteFooter(): JSX.Element {
  return (
    <footer className="site-footer">
      <div>
        <strong>飞梭同步 / TurboSync</strong>
        <p>简单、可控、可自托管的 Rust 文件同步工具。</p>
      </div>
      <nav aria-label="页脚链接">
        <a href="https://github.com/fzf54122/turbo_sync">GitHub</a>
        <a href="https://github.com/fzf54122/turbo_sync#快速开始">README</a>
        <a href="https://github.com/fzf54122/turbo_sync/blob/master/docs/user-guide.zh.md">用户指南</a>
        <a href="https://github.com/fzf54122/turbo_sync/blob/master/LICENSE">MIT License</a>
      </nav>
    </footer>
  );
}
