interface MetricCardProps {
  label: string;
  value: string;
  detail: string;
  accent?: string;
}

export function MetricCard({ label, value, detail }: MetricCardProps) {
  return (
    <section className="metric-card">
      <p>{label}</p>
      <div>
        <strong>{value}</strong>
        <span>{detail}</span>
      </div>
    </section>
  );
}
