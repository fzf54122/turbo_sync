interface StatusBadgeProps {
  label: string;
  tone?: 'green' | 'red' | 'amber' | 'blue' | 'slate';
}

const toneClass = {
  green: 'status-badge status-badge-green',
  red: 'status-badge status-badge-red',
  amber: 'status-badge status-badge-amber',
  blue: 'status-badge status-badge-blue',
  slate: 'status-badge status-badge-slate',
};

export function StatusBadge({ label, tone = 'slate' }: StatusBadgeProps) {
  return (
    <span className={toneClass[tone]}>
      {label}
    </span>
  );
}
