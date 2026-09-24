import { describe, expect, it } from "vitest";
import { cn, fmtLatency, fmtTs, pct } from "./utils";

describe("fmtLatency", () => {
  it("formats sub-second values as ms", () => {
    expect(fmtLatency(2)).toBe("2ms");
    expect(fmtLatency(999)).toBe("999ms");
  });

  it("formats second-plus values as s", () => {
    expect(fmtLatency(1000)).toBe("1.00s");
    expect(fmtLatency(1500)).toBe("1.50s");
  });
});

describe("pct", () => {
  it("computes rounded percentages", () => {
    expect(pct(1, 4)).toBe("25%");
    expect(pct(2, 3)).toBe("67%");
  });

  it("never divides by zero", () => {
    expect(pct(5, 0)).toBe("0%");
  });
});

describe("fmtTs", () => {
  it("renders an ISO timestamp without throwing", () => {
    const out = fmtTs("2026-09-24T00:00:00.000Z");
    expect(out).toContain("2026");
  });
});

describe("cn", () => {
  it("merges class lists", () => {
    expect(cn("a", "b")).toBe("a b");
    expect(cn("px-2", "px-4")).toBe("px-4");
  });
});
