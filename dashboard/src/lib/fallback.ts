/**
 * Backend-with-mock-fallback wrappers (static SPA can demo with no backend).
 *
 * Split out of `api.ts` so `api.ts` stays cycle-free: `mock.ts` imports
 * proto-shaped enums from `api.ts`, and this module imports both.
 * Fail-safe: any network/parse error falls back to redacted mock data.
 */

import {
  dryRun,
  fetchAuditHistory,
  getPolicy,
  publishPolicy,
  queryStats,
} from "./api";
import type {
  AuditEntry,
  DryRunRequest,
  DryRunResponse,
  GetPolicyRequest,
  GetPolicyResponse,
  PublishPolicyRequest,
  PublishPolicyResponse,
  QueryStatsRequest,
  QueryStatsResponse,
} from "./api";
import { mockAuditEntries, mockStats } from "./mock";

export async function fetchAuditHistoryWithFallback(params?: {
  limit?: number;
  org_id?: string;
}): Promise<AuditEntry[]> {
  try {
    const data = await fetchAuditHistory(params);
    if (Array.isArray(data) && data.length > 0) return data;
    return mockAuditEntries.slice(0, params?.limit ?? 50);
  } catch {
    // No backend — static SPA fallback for demo / a11y / Playwright static build
    return mockAuditEntries.slice(0, params?.limit ?? 50);
  }
}

export async function queryStatsWithFallback(
  req: QueryStatsRequest = {},
): Promise<QueryStatsResponse> {
  try {
    return await queryStats(req);
  } catch {
    return mockStats;
  }
}

export async function getPolicyWithFallback(
  req: GetPolicyRequest = {},
): Promise<GetPolicyResponse> {
  try {
    return await getPolicy(req);
  } catch {
    // Fallback: return a static YAML bundle as base64
    const yaml =
      `# algorithco guard policy — static fallback\nversion: v0.1.0-fallback\nrules:\n  - id: deny-rm-rf\n    when: shell_argv contains "rm -rf"\n    action: deny\n    reason: "destructive rm blocked by local rule"` +
      "\n";
    return {
      bundle: {
        version: "v0.1.0-fallback",
        signed_bytes: btoa(yaml),
        sig: btoa("mock-sig"),
      },
      not_modified: false,
    };
  }
}

export async function dryRunWithFallback(
  req: DryRunRequest,
): Promise<DryRunResponse> {
  try {
    return await dryRun(req);
  } catch {
    // Local dry-run mock: pretend we evaluated history and changed 2 decisions
    const evaluated = req.history_ids.length || 12;
    return {
      result: `mock dry-run: bundle ${req.bundle.version} vs ${evaluated} records — 10 allow, 1 ask, 1 deny (no backend)`,
      decisions: Array.from(
        { length: Math.min(evaluated, 5) },
        (_, i) => `record-${i}: ACTION_ALLOW (mock)`,
      ),
      evaluated,
    };
  }
}

export async function publishPolicyWithFallback(
  req: PublishPolicyRequest,
): Promise<PublishPolicyResponse> {
  try {
    return await publishPolicy(req);
  } catch {
    // Mock: daemon would pick up new bundle via polling / /v1/policy
    return { version: req.bundle.version || `v-mock-${Date.now()}`, ok: true };
  }
}
