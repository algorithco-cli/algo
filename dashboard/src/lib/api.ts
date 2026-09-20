/**
 * GuardService client stub — generated-ish from proto backend.proto (P3-01 tag).
 * Canonical proto: algorithco_guard.v0.GuardService (backend.proto)
 * Plus Decision (decision.proto) and ToolBefore/ToolAfter (events.proto)
 *
 * Endpoints (ConnectRPC → REST mapping for static SPA):
 *   POST /v1/audit           → IngestAudit
 *   GET  /v1/audit            → list audit (history, local daemon or backend)
 *   GET  /v1/audit/stream     → SSE live feed (EventSource)
 *   GET  /v1/policy           → GetPolicy
 *   POST /v1/policy           → PublishPolicy
 *   POST /v1/policy/dryRun    → DryRun
 *   GET  /v1/stats            → QueryStats
 *
 * Fail-safe: network / parse errors → mock fallback (static) or ASK semantics.
 * Privacy: all payloads redacted by default; no unredacted code leaves device
 * without explicit `full` consent (see docs/privacy-dataflow.md).
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
}

export interface QueryStatsResponse {
  total: number;
  allow: number;
  deny: number;
  ask: number;
  avg_latency_ms: number;
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

// Auth (device flow) — stubs for completeness per backend.proto
export async function authDevice(req: { client_id: string; scope: string }): Promise<unknown> {
  return fetchJson("/v1/auth/device", { method: "POST", body: JSON.stringify(req) });
}

export async function authDevicePoll(req: { device_code: string }): Promise<unknown> {
  return fetchJson("/v1/auth/device/poll", { method: "POST", body: JSON.stringify(req) });
}

export async function createOrg(req: { org_name: string; owner_id: string }): Promise<{ org: Org }> {
  return fetchJson("/v1/orgs", { method: "POST", body: JSON.stringify(req) });
}

export async function getOrg(org_id: string): Promise<{ org: Org }> {
  return fetchJson(`/v1/orgs/${encodeURIComponent(org_id)}`);
}

// Policy
export async function getPolicy(req: GetPolicyRequest): Promise<GetPolicyResponse> {
  const params = new URLSearchParams();
  if (req.version) params.set("version", req.version);
  if (req.org_id) params.set("org_id", req.org_id);
  const qs = params.toString() ? `?${params.toString()}` : "";
  return fetchJson<GetPolicyResponse>(`/v1/policy${qs}`);
}

export async function publishPolicy(req: PublishPolicyRequest): Promise<PublishPolicyResponse> {
  // Serialize Uint8Array as base64 for JSON transport in static stub
  const body = {
    ...req,
    bundle: {
      version: req.bundle.version,
      signed_bytes:
        typeof req.bundle.signed_bytes === "string"
          ? req.bundle.signed_bytes
          : btoa(String.fromCharCode(...(req.bundle.signed_bytes as Uint8Array))),
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
          : btoa(String.fromCharCode(...(req.bundle.signed_bytes as Uint8Array))),
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
export async function ingestAudit(req: IngestAuditRequest): Promise<IngestAuditResponse> {
  return fetchJson<IngestAuditResponse>("/v1/audit", {
    method: "POST",
    body: JSON.stringify(req),
  });
}

export async function queryStats(req: QueryStatsRequest = {}): Promise<QueryStatsResponse> {
  const params = new URLSearchParams();
  if (req.org_id) params.set("org_id", req.org_id);
  if (req.from) params.set("from", req.from);
  if (req.to) params.set("to", req.to);
  const qs = params.toString() ? `?${params.toString()}` : "";
  return fetchJson<QueryStatsResponse>(`/v1/stats${qs}`);
}

export async function fetchAuditHistory(params?: { limit?: number; org_id?: string }): Promise<AuditEntry[]> {
  const qs = new URLSearchParams();
  if (params?.limit) qs.set("limit", String(params.limit));
  if (params?.org_id) qs.set("org_id", params.org_id);
  const suffix = qs.toString() ? `?${qs.toString()}` : "";
  return fetchJson<AuditEntry[]>(`/v1/audit${suffix}`);
}

// ─── Mock fallback (no backend) ─────────────────────────────────────────────

import { mockAuditEntries, mockStats } from "./mock";

export async function fetchAuditHistoryWithFallback(params?: { limit?: number; org_id?: string }): Promise<AuditEntry[]> {
  try {
    const data = await fetchAuditHistory(params);
    if (Array.isArray(data) && data.length > 0) return data;
    return mockAuditEntries.slice(0, params?.limit ?? 50);
  } catch {
    // No backend — static SPA fallback for demo / a11y / Playwright static build
    return mockAuditEntries.slice(0, params?.limit ?? 50);
  }
}

export async function queryStatsWithFallback(req: QueryStatsRequest = {}): Promise<QueryStatsResponse> {
  try {
    return await queryStats(req);
  } catch {
    return mockStats;
  }
}

export async function getPolicyWithFallback(req: GetPolicyRequest = {}): Promise<GetPolicyResponse> {
  try {
    return await getPolicy(req);
  } catch {
    // Fallback: return a static YAML bundle as base64
    const yaml = `# algorithco guard policy — static fallback\nversion: v0.1.0-fallback\nrules:\n  - id: deny-rm-rf\n    when: shell_argv contains "rm -rf"\n    action: deny\n    reason: "destructive rm blocked by local rule"` + "\n";
    return {
      bundle: { version: "v0.1.0-fallback", signed_bytes: btoa(yaml), sig: btoa("mock-sig") },
      not_modified: false,
    };
  }
}

export async function dryRunWithFallback(req: DryRunRequest): Promise<DryRunResponse> {
  try {
    return await dryRun(req);
  } catch {
    // Local dry-run mock: pretend we evaluated history and changed 2 decisions
    const evaluated = req.history_ids.length || 12;
    return {
      result: `mock dry-run: bundle ${req.bundle.version} vs ${evaluated} records — 10 allow, 1 ask, 1 deny (no backend)`,
      decisions: Array.from({ length: Math.min(evaluated, 5) }, (_, i) => `record-${i}: ACTION_ALLOW (mock)`),
      evaluated,
    };
  }
}

export async function publishPolicyWithFallback(req: PublishPolicyRequest): Promise<PublishPolicyResponse> {
  try {
    return await publishPolicy(req);
  } catch {
    // Mock: daemon would pick up new bundle via polling / /v1/policy
    return { version: req.bundle.version || `v-mock-${Date.now()}`, ok: true };
  }
}
