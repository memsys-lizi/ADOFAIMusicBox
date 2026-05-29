interface StatPillProps {
  label: string;
  value: string;
}

export function StatPill({ label, value }: StatPillProps) {
  return (
    <span className="stat-pill">
      <span>{label}</span>
      <strong>{value}</strong>
    </span>
  );
}
