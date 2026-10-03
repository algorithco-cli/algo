/* web — unified Guard dashboard client (Phase 3, dashboard plan).
 * Single API surface for dashboard routes. Transport reuses billing.ts
 * (apiFetch/apiPath/BillingApiError/token storage) — one env var
 * (VITE_BACKEND_URL), one /api proxy semantic, no duplicated helpers.
 *
 * Backend contract: backend/src/main.rs dashboard shims + proto
 * algorithco_guard.v0 (AuditRecord, ListAudit, SubscribeAudit, QueryStats
 * series). All payloads redacted by default; failures throw BillingApiError
 * (status 0 = unreachable) — callers map errors to ask/mock, never allow.
 */

import {
  BillingApiError,
  apiFetch,
  apiPath,
  loadStoredOrgId,
  loadStoredToken,
  saveStoredOrgId,
} from "./billing";

/* ---------- proto-mirroring types (REST shim, snake_case) ---------- */

/** Numeric Action enum (decision.proto): 1 allow, 2 deny, 3 ask. */
export type DecisionAction = 1 | 2 | 3;

/** Numeric ToolKind enum (events.proto): 1 shell … 6 other. */
export type ToolKindNumber = 1 | 2 | 3 | 4 | 5 | 6;

export interface AuditDecisionView {
  action: number;
  reason: string;
  confidence_0_1: number;
  source_level: number;
  latency_ms: number;
  policy_version: string;
  trace_id: string;
}

export interface AuditRecordView {
  trace_id: string;
  event_id: string;
  /** RFC3339 ingest time. */
  timestamp: string;
  tool_kind: number;
  redacted_payload: string;
  decision: AuditDecisionView;
  privacy_mode: number;
  org_id: string;
}

export interface ListAuditParams {
  org_id?: string;
  limit?: number;
  cursor?: string;
  decision?: "allow" | "deny" | "ask" | string;
  tool_kind?: string;
  from?: string;
  to?: string;
}

export interface ListAuditResponse {
  records: AuditRecordView[];
  next_cursor: string;
  total: number;
  truncated: boolean;
}

export interface StatsBucketView {
  bucket_start: string;
  total: number;
  allow: number;
  deny: number;
  ask: number;
  avg_latency_ms: number;
}

export interface PerUserStatsView {
  user_id: string;
  total: number;
  allow: number;
  deny: number;
  ask: number;
}

export interface PerProjectStatsView {
  project: string;
  total: number;
  allow: number;
  deny: number;
  ask: number;
}

export interface QueryStatsParams {
  org_id?: string;
  from?: string;
  to?: string;
  granularity?: "day" | "hour" | string;
  limit?: number;
  top_n?: number;
}

export interface QueryStatsResponse {
  total: number;
  allow: number;
  deny: number;
  ask: number;
  avg_latency_ms: number;
  buckets?: StatsBucketView[];
  per_user?: PerUserStatsView[];
  per_project?: PerProjectStatsView[];
  truncated?: boolean;
}

export interface OrgView {
  org_id: string;
  org_name: string;
  owner_id: string;
  created_at: string;
}

export interface PolicyBundleView {
  version: string;
  signed_bytes_b64?: string;
  sig_b64?: string;
  content?: string;
}

export interface GuardProfile {
  subject: string;
  provider: string;
  name: string | null;
  email: string | null;
  issuedAt: number | null;
  expiresAt: number | null;
}

/* ---------- session (reuses billing storage; never logs values) ---------- */

export function guardToken(): string {
  return loadStoredToken();
}

export function guardOrgId(): string {
  return loadStoredOrgId();
}

/** Decode display-only JWT claims. Authorization still belongs to backend. */
export function sessionProfileFromToken(
  token: string,
  accountMode = false,
): GuardProfile | null {
  const payload = token.split(".")[1];
  if (!payload) return null;
  try {
    const normalized = payload.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
    const claims = JSON.parse(atob(padded)) as Record<string, unknown>;
    const provider =
      typeof claims.provider === "string"
        ? claims.provider
        : accountMode
          ? "algorithco"
          : null;
    if (typeof claims.sub !== "string" || !claims.sub || !provider) {
      return null;
    }
    return {
      subject: claims.sub,
      provider,
      name:
        typeof claims.name === "string"
          ? claims.name
          : typeof claims.login === "string"
            ? claims.login
            : null,
      email: typeof claims.email === "string" ? claims.email : null,
      issuedAt: typeof claims.iat === "number" ? claims.iat : null,
      expiresAt: typeof claims.exp === "number" ? claims.exp : null,
    };
  } catch {
    return null;
  }
}

/* ---------- fetchers (all authed; paths carry /api for the dev proxy) ---------- */

function qs(params: Record<string, string | number | undefined>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== "") p.set(k, String(v));
  }
  const s = p.toString();
  return s ? `?${s}` : "";
}

export function backendHealth(): Promise<{ status: string; service?: string }> {
  // /health is open (no token) and exists on both proxy + direct base.
  return apiFetch("/api/health", null);
}

export function listAudit(
  params: ListAuditParams,
  token: string,
): Promise<ListAuditResponse> {
  return apiFetch<ListAuditResponse>(
    `/api/v1/audit${qs({ ...params })}`,
    token,
  );
}

export function queryStats(
  params: QueryStatsParams,
  token: string,
): Promise<QueryStatsResponse> {
  return apiFetch<QueryStatsResponse>(
    `/api/v1/stats${qs({ ...params })}`,
    token,
  );
}

export function getOrg(
  orgId: string,
  token: string,
): Promise<{ org: OrgView }> {
  return apiFetch(`/api/v1/orgs/${encodeURIComponent(orgId)}`, token);
}

export function listOrgs(
  token: string,
): Promise<{ orgs: OrgView[]; total: number }> {
  return apiFetch("/api/v1/orgs", token);
}

export async function createOrg(
  orgName: string,
  token: string,
): Promise<OrgView> {
  const org = await apiFetch<OrgView>("/api/v1/orgs", token, {
    method: "POST",
    body: JSON.stringify({ org_name: orgName }),
  });
  saveStoredOrgId(org.org_id);
  return org;
}

export function getPolicy(
  version: string,
  orgId: string,
  token: string,
): Promise<{ version: string; signed_bytes_b64: string; sig_b64: string }> {
  return apiFetch(
    `/api/v1/policy/${encodeURIComponent(version)}?org_id=${encodeURIComponent(orgId)}`,
    token,
  );
}

export function publishPolicy(
  args: { org_id: string; content?: string } & Partial<PolicyBundleView>,
  token: string,
): Promise<{ version: string; ok: boolean }> {
  return apiFetch("/api/v1/policy/publish", token, {
    method: "POST",
    body: JSON.stringify(args),
  });
}

export function dryRunPolicy(
  args: { org_id: string; bundle?: unknown; history_ids?: string[] },
  token: string,
): Promise<{ result: string; decisions: string[]; evaluated: number }> {
  return apiFetch("/api/v1/policy/dry-run", token, {
    method: "POST",
    body: JSON.stringify(args),
  });
}

/* ---------- SSE burst parse (auth via fetch; EventSource has no headers) ---------- */

export interface StreamBurst {
  records: AuditRecordView[];
  /** Highest resume id seen (empty when no records). */
  lastId: string;
  ready: boolean;
}

/**
 * Parse one bounded-burst `text/event-stream` body from GET /v1/audit/stream.
 * Pure + total: malformed frames are skipped (fail-safe: partial data, never
 * throw on backend framing quirks — callers still validate shape).
 */
export function parseSseBurst(text: string): StreamBurst {
  const records: AuditRecordView[] = [];
  let lastId = "";
  let ready = false;
  for (const frame of text.split("\n\n")) {
    let id = "";
    let event = "";
    const dataLines: string[] = [];
    for (const line of frame.split("\n")) {
      if (line.startsWith("id:")) id = line.slice(3).trim();
      else if (line.startsWith("event:")) event = line.slice(6).trim();
      else if (line.startsWith("data:"))
        dataLines.push(line.slice(5).trimStart());
      // ": comment" keep-alives and "retry:" directives are ignored.
    }
    if (event === "ready") {
      ready = true;
      continue;
    }
    if (event !== "record" || dataLines.length === 0) continue;
    try {
      const parsed = JSON.parse(dataLines.join("\n")) as AuditRecordView;
      if (parsed && typeof parsed.trace_id === "string") {
        records.push(parsed);
        if (id) lastId = id;
      }
    } catch {
      /* skip malformed frame */
    }
  }
  return { records, lastId, ready };
}

/** Fetch one stream burst with auth (cursor resumes via Last-Event-ID). */
export async function fetchAuditStream(
  params: ListAuditParams,
  token: string,
): Promise<StreamBurst> {
  const url = apiPath(`/api/v1/audit/stream${qs({ ...params })}`);
  let res: Response;
  try {
    res = await fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : {},
    });
  } catch {
    throw new BillingApiError(
      0,
      "Backend unreachable (is algo-backend running on http://127.0.0.1:8080?).",
      null,
    );
  }
  if (!res.ok) {
    let message = `Stream failed (${res.status})`;
    try {
      const body = (await res.json()) as { error?: unknown };
      if (typeof body.error === "string" && body.error) message = body.error;
    } catch {
      /* keep default */
    }
    throw new BillingApiError(res.status, message, null);
  }
  return parseSseBurst(await res.text());
}

/* ---------- react-query keys (stable, serializable) ---------- */

export const guardKeys = {
  health: ["guard", "health"] as const,
  audit: (p: ListAuditParams) => ["guard", "audit", { ...p }] as const,
  stats: (p: QueryStatsParams) => ["guard", "stats", { ...p }] as const,
  org: (id: string) => ["guard", "org", id] as const,
  orgs: ["guard", "orgs"] as const,
};
