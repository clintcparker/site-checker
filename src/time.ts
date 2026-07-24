/**
 * Age of the last completed check, rendered for the "Last checked" column.
 * Both arguments are epoch milliseconds; `now` is passed in rather than read
 * so this stays pure and testable.
 */
export function formatSince(checkedAt: number | null, now: number): string {
  if (checkedAt === null) return "—";

  const seconds = Math.max(0, Math.floor((now - checkedAt) / 1000));
  if (seconds < 60) return `${seconds}s ago`;

  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;

  return `${Math.floor(minutes / 60)}h ago`;
}
