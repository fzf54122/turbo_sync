import { stabilityNotes } from '../data/siteContent';

export function StabilityNotice(): JSX.Element {
  return (
    <section className="content-section stability-section" aria-labelledby="stability-title">
      <div>
        <span className="section-kicker">TRUST BOUNDARIES</span>
        <h2 id="stability-title">清楚边界，比承诺一切更可靠。</h2>
      </div>
      <ul>
        {stabilityNotes.map((note) => (
          <li key={note}>{note}</li>
        ))}
      </ul>
    </section>
  );
}
