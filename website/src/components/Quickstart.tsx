import { commandGroups } from '../data/siteContent';

export function Quickstart(): JSX.Element {
  return (
    <section className="content-section quickstart-section" id="quickstart" aria-labelledby="quickstart-title">
      <div className="section-heading">
        <span className="section-kicker">QUICKSTART</span>
        <h2 id="quickstart-title">从一个目录、一台目标设备开始。</h2>
        <p>官网只保留最短路径；完整命令和场景说明请查看仓库 README 与用户指南。</p>
      </div>

      <div className="command-grid">
        {commandGroups.map((group) => (
          <article key={group.label} className="command-card">
            <div className="command-card-header">
              <h3>{group.label}</h3>
              <span>{group.commands.length} step{group.commands.length > 1 ? 's' : ''}</span>
            </div>
            <pre aria-label={`${group.label} 命令示例`}>
              {group.commands.map((command) => `$ ${command}`).join('\n')}
            </pre>
            <p>{group.note}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
