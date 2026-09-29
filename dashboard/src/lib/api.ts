/**
 * GuardService client stub — generated-ish from proto backend.proto (P3-01 tag).
 * Canonical proto: algorithco_guard.v0.GuardService (backend.proto)
 * Plus Decision (decision.proto) and ToolBefore/ToolAfter (events.proto)
 *
 * Endpoints (ConnectRPC → REST mapping for static SPA):
 *   POST /v1/audit           → IngestAudit
 *   GET  /v1/audit            → ListAudit (limit/cursor/decision/tool_kind/from/to/org_id)
 *   GET  /v1/audit/stream     → SubscribeAudit (SSE live feed, EventSource)
 *   GET  /v1/policy           → GetPolicy
 *   POST /v1/policy           → PublishPolicy
 *   POST /v1/policy/dryRun    → DryRun
 *   GET  /v1/stats            → QueryStats (granularity/limit/top_n; buckets/per_user/per_project)
 *
 * Contract: proto/algorithco_guard/v0/backend.proto (Phase 1 dashboard:
 * AuditRecord, ListAudit, SubscribeAudit, QueryStats series). This stub
 * mirrors the generated shape; repeated proto fields are `[]` (empty = none).
 *
 * Fail-safe: network / parse errors → mock fallback (static) or ASK semantics.
 * Privacy: all payloads redacted by default; no unredacted code leaves device
 * without explicit `full` consent.
 *
 * TODO: replace with buf-generated TS client when proto tag P3-01 is published.
 * Consumers pin exact proto version per AGENTS.md contracts-first.
 */

// ─── Proto enums (decision.proto §) ──────────────────────────────────────────

export enum Action {
  ACTION_UNSPECIFIED = 0,
  ACTION_ALLOW = 1,
  ACTION_DENY = 2,
  ACTION_ASK = 3,
}

export enum SourceLevel {
  SOURCE_LEVEL_UNSPECIFIED = 0,
  SOURCE_LEVEL_RULE = 1,
  SOURCE_LEVEL_CACHE = 2,
  SOURCE_LEVEL_LOCAL_MODEL = 3,
  SOURCE_LEVEL_JEV = 4,
  SOURCE_LEVEL_FALLBACK = 5,
}

export enum ToolKind {
  TOOL_KIND_UNSPECIFIED = 0,
  TOOL_KIND_SHELL = 1,
  TOOL_KIND_EDIT = 2,
  TOOL_KIND_WRITE = 3,
  TOOL_KIND_READ = 4,
  TOOL_KIND_NET = 5,
  TOOL_KIND_OTHER = 6,
}

export enum PrivacyMode {
  PRIVACY_MODE_UNSPECIFIED = 0,
  PRIVACY_MODE_LOCAL_ONLY = 1,
  PRIVACY_MODE_REDACTED = 2,
  PRIVACY_MODE_FULL = 3,
}

export enum RoleType {
  ROLE_TYPE_UNSPECIFIED = 0,
  ROLE_TYPE_ADMIN = 1,
  ROLE_TYPE_MEMBER = 2,
  ROLE_TYPE_VIEWER = 3,
}

// ─── Proto messages (decision.proto, events.proto, backend.proto) ────────────

export interface Decision {
  action: Action;
  reason: string;
  confidence_0_1: number;
  source_level: SourceLevel;
  latency_ms: number;
  policy_version: string;
  trace_id: string;
}

export interface AgentIdentity {
  agent_type?: string;
  agent_version?: string;
  session_id: string;
  working_dir: string;
}

export interface ToolBefore {
  event_id: string;
  timestamp: string; // ISO 8601 (proto google.protobuf.Timestamp → RFC3339)
  agent: AgentIdentity;
  tool_kind: ToolKind;
  redacted_payload: string;
  privacy_mode: PrivacyMode;
  shell_argv: string[];
  file_path?: string;
}

export interface ToolAfter {
  event_id: string;
  timestamp: string;
  tool_kind: ToolKind;
  exit_code?: number;
  redacted_output: string;
  latency_ms: number;
  error: string;
}

export interface Org {
  org_id: string;
  name: string;
  owner_id: string;
  created_at: string;
}

export interface Team {
  team_id: string;
  org_id: string;
  name: string;
}

export interface RoleAssignment {
  user_id: string;
  org_id: string;
  role: RoleType;
}

export interface PolicyBundle {
  version: string;
  signed_bytes: Uint8Array | string; // base64 string over JSON for static stub
  sig: Uint8Array | string;
}

export interface GetPolicyRequest {
  version?: string;
  org_id?: string;
}

export interface GetPolicyResponse {
  bundle: PolicyBundle;
  not_modified: boolean;
}

export interface PublishPolicyRequest {
  bundle: PolicyBundle;
  org_id: string;
}

export interface PublishPolicyResponse {
  version: string;
  ok: boolean;
}

export interface DryRunRequest {
  bundle: PolicyBundle;
  history_ids: string[];
  org_id: string;
}

export interface DryRunResponse {
  result: string;
  decisions: string[];
  evaluated: number;
}

export interface IngestAuditRequest {
  redacted_event: string;
  decision: string;
  latency_ms: number;
  trace_id: string;
  org_id: string;
}

export interface IngestAuditResponse {
  ok: boolean;
  trace_id: string;
}

export interface QueryStatsRequest {
  org_id?: string;
  from?: string;
  to?: string;
  // "day" (default) | "hour"; unknown → server falls back to "day".
  granularity?: string;
  // Max buckets; 0/omitted = server default (server caps, sets truncated).
  limit?: number;
  // Max per-user/per-project rows each; 0/omitted = server default.
  top_n?: number;
}

export interface StatsBucket {
  bucket_start: string; // RFC3339
  total: number;
  allow: number;
  deny: number;
  ask: number;
  avg_latency_ms: number;
}

export interface PerUserStats {
  user_id: string;
  total: number;
  allow: number;
  deny: number;
  ask: number;
}

export interface PerProjectStats {
  project: string;
  total: number;
  allow: number;
  deny: number;
  ask: number;
}

export interface QueryStatsResponse {
  total: number;
  allow: number;
  deny: number;
  ask: number;
  avg_latency_ms: number;
  // Series (Phase 1 contract). Optional in this hand stub so existing mocks
  // compile; the wire always sends them (empty = none/capped → truncated).
  buckets?: StatsBucket[];
  per_user?: PerUserStats[];
  per_project?: PerProjectStats[];
  truncated?: boolean;
}

// Dashboard audit record (proto AuditRecord). `timestamp` is RFC3339 over the
// JSON/REST shim (proto google.protobuf.Timestamp on the wire).
export interface AuditRecord {
  trace_id: string;
  event_id: string;
  timestamp: string;
  tool_kind: ToolKind;
  redacted_payload: string;
  decision: Decision;
  privacy_mode: PrivacyMode;
  org_id: string;
}

export interface ListAuditRequest {
  org_id?: string;
  limit?: number;
  cursor?: string;
  // "allow"|"deny"|"ask"; omitted = all. Unknown values match nothing.
  decision?: string;
  tool_kind?: ToolKind;
  from?: string; // RFC3339
  to?: string; // RFC3339
}

export interface ListAuditResponse {
  records: AuditRecord[];
  next_cursor: string;
  total: number;
  truncated: boolean;
}

export interface SubscribeAuditRequest {
  org_id?: string;
  cursor?: string;
}

export interface SubscribeAuditResponse {
  record: AuditRecord;
  cursor: string;
}

// Extended audit entry for dashboard table (joins Decision + ToolBefore + explain fields)
export interface AuditEntry {
  trace_id: string;
  event_id: string;
  timestamp: string; // ISO
  tool_kind: ToolKind;
  redacted_payload: string;
  decision: Decision;
  // denormalized for table convenience
  action: Action;
  reason: string;
  confidence: number;
  source: SourceLevel;
  latency_ms: number;
}

// ─── Config ──────────────────────────────────────────────────────────────────

const API_BASE: string =
  (import.meta.env.VITE_API_BASE as string | undefined) ?? "";

// ─── Helpers ─────────────────────────────────────────────────────────────────

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${API_BASE}${path}`;
  const res = await fetch(url, {
    headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
    ...init,
  });
  if (!res.ok) {
    throw new Error(`HTTP ${res.status} ${res.statusText} for ${path}`);
  }
  return (await res.json()) as T;
}

// ─── GuardService wrappers ───────────────────────────────────────────────────

// Auth (email accounts + provider sessions) — backend mints short-lived
// session JWTs; callers attach them as `Authorization: Bearer <token>`.
export interface EmailSession {
  provider: string;
  email: string;
  name: string | null;
  session_token: string;
  expires_in: number;
}

export async function authSignup(req: {
  email: string;
  password: string;
  name?: string;
}): Promise<EmailSession> {
  return fetchJson("/v1/auth/signup", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function authLogin(req: {
  email: string;
  password: string;
}): Promise<EmailSession> {
  return fetchJson("/v1/auth/login", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function createOrg(req: {
  org_name: string;
  owner_id: string;
}): Promise<{ org: Org }> {
  return fetchJson("/v1/orgs", { method: "POST", body: JSON.stringify(req) });
}

export async function getOrg(org_id: string): Promise<{ org: Org }> {
  return fetchJson(`/v1/orgs/${encodeURIComponent(org_id)}`);
}

// Policy
export async function getPolicy(
  req: GetPolicyRequest,
): Promise<GetPolicyResponse> {
  const params = new URLSearchParams();
  if (req.version) params.set("version", req.version);
  if (req.org_id) params.set("org_id", req.org_id);
  const qs = params.toString() ? `?${params.toString()}` : "";
  return fetchJson<GetPolicyResponse>(`/v1/policy${qs}`);
}

export async function publishPolicy(
  req: PublishPolicyRequest,
): Promise<PublishPolicyResponse> {
  // Serialize Uint8Array as base64 for JSON transport in static stub
  const body = {
    ...req,
    bundle: {
      version: req.bundle.version,
      signed_bytes:
        typeof req.bundle.signed_bytes === "string"
          ? req.bundle.signed_bytes
          : btoa(
              String.fromCharCode(...(req.bundle.signed_bytes as Uint8Array)),
            ),
      sig:
        typeof req.bundle.sig === "string"
          ? req.bundle.sig
          : btoa(String.fromCharCode(...(req.bundle.sig as Uint8Array))),
    },
  };
  return fetchJson<PublishPolicyResponse>("/v1/policy", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function dryRun(req: DryRunRequest): Promise<DryRunResponse> {
  const body = {
    ...req,
    bundle: {
      version: req.bundle.version,
      signed_bytes:
        typeof req.bundle.signed_bytes === "string"
          ? req.bundle.signed_bytes
          : btoa(
              String.fromCharCode(...(req.bundle.signed_bytes as Uint8Array)),
            ),
      sig:
        typeof req.bundle.sig === "string"
          ? req.bundle.sig
          : btoa(String.fromCharCode(...(req.bundle.sig as Uint8Array))),
    },
  };
  return fetchJson<DryRunResponse>("/v1/policy/dryRun", {
    method: "POST",
    body: JSON.stringify(body),
  });
}

// Audit
export async function ingestAudit(
  req: IngestAuditRequest,
): Promise<IngestAuditResponse> {
  return fetchJson<IngestAuditResponse>("/v1/audit", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function queryStats(
  req: QueryStatsRequest = {},
): Promise<QueryStatsResponse> {
  const params = new URLSearchParams();
  if (req.org_id) params.set("org_id", req.org_id);
  if (req.from) params.set("from", req.from);
  if (req.to) params.set("to", req.to);
  if (req.granularity) params.set("granularity", req.granularity);
  if (req.limit) params.set("limit", String(req.limit));
  if (req.top_n) params.set("top_n", String(req.top_n));
  const qs = params.toString() ? `?${params.toString()}` : "";
  return fetchJson<QueryStatsResponse>(`/v1/stats${qs}`);
}

export interface AuditHistoryParams extends ListAuditRequest {}

export async function listAudit(
  req: ListAuditRequest = {},
): Promise<ListAuditResponse> {
  const qs = new URLSearchParams();
  if (req.org_id) qs.set("org_id", req.org_id);
  if (req.limit) qs.set("limit", String(req.limit));
  if (req.cursor) qs.set("cursor", req.cursor);
  if (req.decision) qs.set("decision", req.decision);
  if (req.tool_kind) qs.set("tool_kind", String(req.tool_kind));
  if (req.from) qs.set("from", req.from);
  if (req.to) qs.set("to", req.to);
  const suffix = qs.toString() ? `?${qs.toString()}` : "";
  return fetchJson<ListAuditResponse>(`/v1/audit${suffix}`);
}

function recordToEntry(r: AuditRecord): AuditEntry {
  return {
    trace_id: r.trace_id,
    event_id: r.event_id,
    timestamp: r.timestamp,
    tool_kind: r.tool_kind,
    redacted_payload: r.redacted_payload,
    decision: r.decision,
    action: r.decision.action,
    reason: r.decision.reason,
    confidence: r.decision.confidence_0_1,
    source: r.decision.source_level,
    latency_ms: r.decision.latency_ms,
  };
}

export async function fetchAuditHistory(params?: {
  limit?: number;
  org_id?: string;
  cursor?: string;
  decision?: string;
  tool_kind?: ToolKind;
  from?: string;
  to?: string;
}): Promise<AuditEntry[]> {
  const res = await listAudit(params ?? {});
  if (Array.isArray(res.records)) return res.records.map(recordToEntry);
  // Pre-Phase-2 backend returns a bare array; accept it fail-open-to-mock
  // (callers fall back to mock on empty, never to allow).
  if (Array.isArray(res as unknown)) return res as unknown as AuditEntry[];
  throw new Error("unexpected /v1/audit shape");
}

// ─── Mock fallback (no backend) ─────────────────────────────────────────────
// Moved to `./fallback.ts` (api.ts must stay cycle-free: mock.ts imports the
// proto-shaped enums above, and fallback.ts imports both).
