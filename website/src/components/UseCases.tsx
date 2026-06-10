import { useCases } from '../data/siteContent';

export function UseCases(): JSX.Element {
  return (
    <section className="content-section usecase-section" aria-labelledby="usecases-title">
      <div className="section-heading align-left">
        <span className="section-kicker">BUILT FOR</span>
        <h2 id="usecases-title">适合自己掌控设备和数据路线的人。</h2>
      </div>
      <div className="usecase-grid">
        {useCases.map((useCase) => (
          <article key={useCase.title}>
            <h3>{useCase.title}</h3>
            <p>{useCase.description}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
