import { describe, expect, it } from "vitest";
import {
  mockEntries,
  mockPerProject,
  mockPerUser,
  mockStats,
  mockTimeSeries,
  savedMinutes,
} from "./mock";

describe("dashboard mock fixtures", () => {
  it("entries are redacted and cover all three actions", () => {
    expect(mockEntries.length).toBeGreaterThan(0);
    const actions = new Set(mockEntries.map((e) => e.action));
    expect(actions).toEqual(new Set([1, 2, 3]));
    for (const e of mockEntries) {
      expect(e.trace_id).toBeTruthy();
      expect(e.redacted_payload).not.toMatch(/ghp_|sk-live|AKIA/);
    }
  });

  it("mockStats matches entry counts", () => {
    expect(mockStats.total).toBe(mockEntries.length);
    expect(mockStats.allow + mockStats.ask + mockStats.deny).toBe(
      mockStats.total,
    );
  });

  it("per-user/project rows are static demo", () => {
    expect(mockPerUser.length).toBeGreaterThan(0);
    expect(mockPerProject.length).toBeGreaterThan(0);
  });

  it("time series is deterministic (14 daily points)", () => {
    expect(mockTimeSeries).toHaveLength(14);
    for (let i = 1; i < mockTimeSeries.length; i++) {
      expect(mockTimeSeries[i].ts - mockTimeSeries[i - 1].ts).toBe(86400);
    }
  });

  it("savedMinutes documents placeholder math", () => {
    expect(savedMinutes(20, 2, 1)).toBeCloseTo(2.5, 5);
  });
});
