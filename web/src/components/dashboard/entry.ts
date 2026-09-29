/* web — dashboard entry model (Phase 4, dashboard plan).
 * Denormalized row joining guard.ts AuditRecordView for react-table.
 * Numeric enums mirror proto (decision.proto / events.proto).
 * Pure + tested (entry.test.ts). No Tailwind: web CSS classes only.
 */

import type { AuditRecordView } from "../../lib/guard";

/** Numeric Action: 1 allow, 2 deny, 3 ask (0/unknown → ask). */
export type DashAction = 1 | 2 | 3;

export interface DashEntry {
  trace_id: string;
  event_id: string;
  timestamp: string;
  tool_kind: number;
  redacted_payload: string;
  action: DashAction;
  reason: string;
  confidence: number;
  source: number;
  latency_ms: number;
  policy_version: string;
}

function toAction(n: number): DashAction {
  return n === 1 || n === 2 ? n : 3;
}

/** Backend view → table row. Unknown/missing decisions resolve to ask. */
export function recordToEntry(r: AuditRecordView): DashEntry {
  const d = r.decision ?? {
    action: 3,
    reason: "",
    confidence_0_1: 0,
    source_level: 0,
    latency_ms: 0,
    policy_version: "",
    trace_id: r.trace_id,
  };
  return {
    trace_id: r.trace_id,
    event_id: r.event_id,
    timestamp: r.timestamp,
    tool_kind: r.tool_kind,
    redacted_payload: r.redacted_payload,
    action: toAction(d.action),
    reason: d.reason ?? "",
    confidence: typeof d.confidence_0_1 === "number" ? d.confidence_0_1 : 0,
    source: d.source_level ?? 0,
    latency_ms: d.latency_ms ?? 0,
    policy_version: d.policy_version ?? "",
  };
}

export function actionLabel(a: number): "allow" | "deny" | "ask" {
  if (a === 1) return "allow";
  if (a === 2) return "deny";
  return "ask";
}

export function actionBadgeClass(a: number): string {
  if (a === 1) return "badge badge-allow";
  if (a === 2) return "badge badge-deny";
  return "badge badge-ask";
}

export function toolKindLabel(k: number): string {
  switch (k) {
    case 1:
      return "SHELL";
    case 2:
      return "EDIT";
    case 3:
      return "WRITE";
    case 4:
      return "READ";
    case 5:
      return "NET";
    default:
      return "OTHER";
  }
}

export function sourceLabel(s: number): string {
  switch (s) {
    case 1:
      return "RULE";
    case 2:
      return "CACHE";
    case 3:
      return "LOCAL_MODEL";
    case 4:
      return "JEV";
    case 5:
      return "FALLBACK";
    default:
      return "UNKNOWN";
  }
}

export function fmtLatency(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

export function fmtTs(iso: string): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return iso;
  return new Date(t).toLocaleString();
}

export function pct(n: number, total: number): string {
  if (!total) return "0%";
  return `${Math.round((n / total) * 100)}%`;
}

/** `algo why` one-liner for a row (action+reason+confidence+source+latency). */
export function whyLine(e: DashEntry): string {
  return `${actionLabel(e.action)} · ${e.reason || "no reason recorded"} · conf ${e.confidence.toFixed(2)} · ${sourceLabel(e.source)} · ${fmtLatency(e.latency_ms)}`;
}
