import { actionBadgeClass, actionLabel } from "./entry";

/**
 * Decision badge — web CSS only (.badge + .badge-allow/ask/deny from
 * styles.css). Decision colors exclusive to allow/ask/deny (Variant 1).
 */
export function DecisionBadge({ action }: { action: number }): JSX.Element {
  return (
    <span className={actionBadgeClass(action)}>{actionLabel(action)}</span>
  );
}

/** Compact dot for live-feed rows (same exclusive decision colors). */
export function ActionDot({ action }: { action: number }): JSX.Element {
  const color =
    action === 1
      ? "var(--ag-allow)"
      : action === 2
        ? "var(--ag-deny)"
        : "var(--ag-ask)";
  return (
    <span className="dash-dot" style={{ background: color }} aria-hidden />
  );
}
