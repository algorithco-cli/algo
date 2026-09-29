import { describe, expect, it } from "vitest";
import type { AuditRecordView } from "../../lib/guard";
import {
  actionBadgeClass,
  actionLabel,
  fmtLatency,
  fmtTs,
  pct,
  recordToEntry,
  sourceLabel,
  toolKindLabel,
  whyLine,
} from "./entry";

const VIEW: AuditRecordView = {
  trace_id: "t-1",
  event_id: "e-1",
  timestamp: "2026-09-28T12:00:00.000Z",
  tool_kind: 1,
  redacted_payload: "redacted ls",
  decision: {
    action: 2,
    reason: "blocked",
    confidence_0_1: 0.5,
    source_level: 1,
    latency_ms: 7,
    policy_version: "3",
    trace_id: "t-1",
  },
  privacy_mode: 2,
  org_id: "org-1",
};

describe("recordToEntry", () => {
  it("maps backend view to table row", () => {
    const e = recordToEntry(VIEW);
    expect(e.action).toBe(2);
    expect(e.reason).toBe("blocked");
    expect(e.confidence).toBe(0.5);
    expect(e.source).toBe(1);
    expect(e.latency_ms).toBe(7);
  });

  it("unknown action resolves to ask (fail-safe)", () => {
    const e = recordToEntry({
      ...VIEW,
      decision: { ...VIEW.decision, action: 0 },
    });
    expect(e.action).toBe(3);
    expect(actionLabel(e.action)).toBe("ask");
  });
});

describe("labels", () => {
  it("action labels + badge classes stay exclusive", () => {
    expect(actionLabel(1)).toBe("allow");
    expect(actionLabel(2)).toBe("deny");
    expect(actionLabel(99)).toBe("ask");
    expect(actionBadgeClass(1)).toBe("badge badge-allow");
    expect(actionBadgeClass(2)).toBe("badge badge-deny");
    expect(actionBadgeClass(0)).toBe("badge badge-ask");
  });

  it("tool/source labels with OTHER/UNKNOWN fallback", () => {
    expect(toolKindLabel(1)).toBe("SHELL");
    expect(toolKindLabel(5)).toBe("NET");
    expect(toolKindLabel(0)).toBe("OTHER");
    expect(toolKindLabel(99)).toBe("OTHER");
    expect(sourceLabel(1)).toBe("RULE");
    expect(sourceLabel(5)).toBe("FALLBACK");
    expect(sourceLabel(0)).toBe("UNKNOWN");
  });
});

describe("formatters", () => {
  it("fmtLatency handles ms/s/invalid", () => {
    expect(fmtLatency(7)).toBe("7ms");
    expect(fmtLatency(1500)).toBe("1.50s");
    expect(fmtLatency(-1)).toBe("—");
    expect(fmtLatency(Number.NaN)).toBe("—");
  });

  it("fmtTs passes through garbage", () => {
    expect(fmtTs("not-a-time")).toBe("not-a-time");
    expect(fmtTs("2026-09-28T12:00:00.000Z")).not.toBe(
      "2026-09-28T12:00:00.000Z",
    );
  });

  it("pct guards divide-by-zero", () => {
    expect(pct(1, 4)).toBe("25%");
    expect(pct(0, 0)).toBe("0%");
  });

  it("whyLine renders action+reason+confidence+source+latency", () => {
    const line = whyLine(recordToEntry(VIEW));
    expect(line).toContain("deny");
    expect(line).toContain("blocked");
    expect(line).toContain("0.50");
    expect(line).toContain("RULE");
    expect(line).toContain("7ms");
  });
});
