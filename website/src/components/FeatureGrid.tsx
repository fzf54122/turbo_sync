import { features, type Feature } from '../data/siteContent';

const statusLabel: Record<Feature['status'], string> = {
  stable: '稳定主线',
  supported: '已支持',
  early: '早期验证',
};

export function FeatureGrid(): JSX.Element {
  return (
    <section className="content-section feature-section" aria-labelledby="features-title">
      <div className="section-heading align-left">
        <span className="section-kicker">CAPABILITIES</span>
        <h2 id="features-title">把同步拆成可观测、可验证、可维护的能力。</h2>
      </div>
      <div className="feature-grid">
        {features.map((feature) => (
          <article key={feature.title} className={`feature-card is-${feature.status}`}>
            <span>{statusLabel[feature.status]}</span>
            <h3>{feature.title}</h3>
            <p>{feature.description}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
