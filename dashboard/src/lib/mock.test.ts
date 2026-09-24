import { describe, expect, it } from "vitest";
import { Action } from "./api";
import { mockAuditEntries, mockStats } from "./mock";

describe("mockAuditEntries invariants", () => {
  it("is non-empty with unique trace_ids", () => {
    expect(mockAuditEntries.length).toBeGreaterThan(0);
    const ids = mockAuditEntries.map((e) => e.trace_id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("keeps denormalized fields in sync with nested decision", () => {
    for (const entry of mockAuditEntries) {
      expect(entry.action).toBe(entry.decision.action);
      expect(entry.latency_ms).toBe(entry.decision.latency_ms);
      expect(entry.confidence).toBe(entry.decision.confidence_0_1);
      expect(entry.source).toBe(entry.decision.source_level);
      expect(entry.trace_id).toBe(entry.decision.trace_id);
      expect(entry.reason.length).toBeGreaterThan(0);
      expect(entry.redacted_payload.length).toBeGreaterThan(0);
    }
  });

  it("covers allow, ask, and deny decisions", () => {
    const actions = new Set(mockAuditEntries.map((e) => e.action));
    expect(actions.has(Action.ACTION_ALLOW)).toBe(true);
    expect(actions.has(Action.ACTION_ASK)).toBe(true);
    expect(actions.has(Action.ACTION_DENY)).toBe(true);
  });

  it("never embeds a live secret pattern in redacted payloads", () => {
    const secret =
      /(AKIA[0-9A-Z]{16}|ghp_[A-Za-z0-9]{20,}|-----BEGIN .*PRIVATE KEY-----)/;
    for (const entry of mockAuditEntries) {
      expect(entry.redacted_payload).not.toMatch(secret);
    }
  });
});

describe("mockStats invariants", () => {
  it("totals equal the sum of parts", () => {
    expect(mockStats.total).toBe(
      mockStats.allow + mockStats.deny + mockStats.ask,
    );
  });
});
