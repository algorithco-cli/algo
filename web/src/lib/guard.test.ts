import { describe, expect, it } from "vitest";
import { guardKeys, parseSseBurst } from "./guard";

const RECORD = JSON.stringify({
  trace_id: "t-1",
  event_id: "e-1",
  timestamp: "2026-09-28T12:00:00.000Z",
  tool_kind: 1,
  redacted_payload: "redacted ls",
  decision: {
    action: 1,
    reason: "ok",
    confidence_0_1: 0.9,
    source_level: 1,
    latency_ms: 4,
    policy_version: "3",
    trace_id: "t-1",
  },
  privacy_mode: 2,
  org_id: "org-1",
});

describe("parseSseBurst", () => {
  it("parses record events with resume ids + ready", () => {
    const burst = [
      "retry: 5000",
      "",
      `id: 1\nevent: record\ndata: ${RECORD}`,
      "",
      "event: ready\ndata: {}",
      "",
    ].join("\n");
    const out = parseSseBurst(burst);
    expect(out.records).toHaveLength(1);
    expect(out.records[0].trace_id).toBe("t-1");
    expect(out.records[0].decision.action).toBe(1);
    expect(out.lastId).toBe("1");
    expect(out.ready).toBe(true);
  });

  it("skips malformed frames fail-safe (never throws)", () => {
    const out = parseSseBurst(
      "event: record\ndata: {nope\n\n: keep-alive\n\nevent: record\ndata: []\n\n",
    );
    expect(out.records).toEqual([]);
    expect(out.ready).toBe(false);
  });

  it("empty body yields empty burst", () => {
    expect(parseSseBurst("")).toEqual({
      records: [],
      lastId: "",
      ready: false,
    });
  });
});

describe("guardKeys", () => {
  it("keys are stable and serializable", () => {
    expect(guardKeys.health).toEqual(["guard", "health"]);
    expect(guardKeys.audit({ org_id: "o", limit: 10 })).toEqual([
      "guard",
      "audit",
      { org_id: "o", limit: 10 },
    ]);
    expect(() =>
      JSON.stringify(guardKeys.stats({ granularity: "day" })),
    ).not.toThrow();
  });
});
