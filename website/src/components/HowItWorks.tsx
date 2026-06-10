import { flowSteps } from '../data/siteContent';

export function HowItWorks(): JSX.Element {
  return (
    <section className="content-section flow-section" id="how-it-works" aria-labelledby="flow-title">
      <div className="section-heading">
        <span className="section-kicker">HOW IT WORKS</span>
        <h2 id="flow-title">从控制入口到目标节点，每一步都有明确职责。</h2>
        <p>本机 Agent 负责扫描、监听、索引、计划和传输；GUI/TUI/CLI 只负责把控制权交给你。</p>
      </div>

      <div className="flow-rail">
        {flowSteps.map((step) => (
          <article key={step.label} className="flow-card">
            <span>{step.label}</span>
            <h3>{step.title}</h3>
            <p>{step.description}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
