import { surfaces } from '../data/siteContent';

export function SurfaceShowcase(): JSX.Element {
  return (
    <section className="content-section" id="surfaces" aria-labelledby="surfaces-title">
      <div className="section-heading">
        <span className="section-kicker">THREE SURFACES</span>
        <h2 id="surfaces-title">同一个 Agent，三种操作手感。</h2>
        <p>日常用 GUI，终端用 TUI，自动化用 CLI。不同入口连接同一个本机 Agent，不重复造同步逻辑。</p>
      </div>

      <div className="surface-grid">
        {surfaces.map((surface, index) => (
          <article key={surface.name} className="surface-card">
            <span className="surface-index">0{index + 1}</span>
            <p className="eyebrow">{surface.eyebrow}</p>
            <h3>{surface.name}</h3>
            <strong>{surface.description}</strong>
            <p>{surface.detail}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
