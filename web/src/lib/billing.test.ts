import { afterEach, describe, expect, it, vi } from "vitest";
import {
  BILLING_PLANS,
  type CurrentSubscription,
  PAST_DUE_GRACE_SECONDS,
  type PaidTier,
  type Plan,
  type Subscription,
  annualMonthlyTotalCents,
  annualYearlyTotalCents,
  apiPath,
  formatCentsUsd,
  formatPeriodDate,
  graceSecondsLeft,
  isPaidTier,
  isValidCycle,
  monthlyTotalCents,
  parseBillingSearchParams,
  planByTier,
  previewChange,
  subscriptionAccess,
  validateSeats,
  yearlySavingsCents,
} from "./billing";

function mustPlan(tier: PaidTier): Plan {
  const found = planByTier(BILLING_PLANS, tier);
  if (!found) throw new Error(`missing static plan ${tier}`);
  return found;
}

function activeSub(overrides: Partial<Subscription> = {}): Subscription {
  return {
    id: "sub_1",
    org_id: "org_1",
    tier: "pro",
    status: "active",
    seats: 3,
    cycle: "monthly",
    price_cents: 3600,
    period_start: 1_700_000_000,
    period_end: 1_700_259_200,
    trial_end: null,
    cancel_at_period_end: false,
    scheduled_tier: null,
    provider: "manual",
    ...overrides,
  };
}

const NOW = 1_700_100_000;

describe("price totals", () => {
  const pro = mustPlan("pro");
  const max = mustPlan("max");
  const team = mustPlan("team");

  it("matches the Pricing catalog (pro 12/10, max 29/24, team 49/41)", () => {
    expect([pro.monthly_cents, pro.annual_cents]).toEqual([1200, 1000]);
    expect([max.monthly_cents, max.annual_cents]).toEqual([2900, 2400]);
    expect([team.monthly_cents, team.annual_cents]).toEqual([4900, 4100]);
  });

  it("monthly totals scale per seat", () => {
    expect(monthlyTotalCents(pro, 1)).toBe(1200);
    expect(monthlyTotalCents(pro, 5)).toBe(6000);
    expect(monthlyTotalCents(team, 10)).toBe(49000);
  });

  it("annual totals: monthly charge, full year, savings", () => {
    expect(annualMonthlyTotalCents(pro, 2)).toBe(2000);
    expect(annualYearlyTotalCents(pro, 2)).toBe(24000);
    // (1200 - 1000) * 12 * 2 = 4800
    expect(yearlySavingsCents(pro, 2)).toBe(4800);
    expect(yearlySavingsCents(max, 1)).toBe(6000);
    expect(yearlySavingsCents(team, 1)).toBe(9600);
  });

  it("formats cents to whole dollars without trailing .00", () => {
    expect(formatCentsUsd(1200)).toBe("$12");
    expect(formatCentsUsd(4800)).toBe("$48");
    expect(formatCentsUsd(1050)).toBe("$10.50");
    expect(formatCentsUsd(0)).toBe("$0");
  });

  it("formats period dates, guards bad input", () => {
    expect(formatPeriodDate(1_700_000_000)).toContain("2023");
    expect(formatPeriodDate(0)).toBe("—");
    expect(formatPeriodDate(Number.NaN)).toBe("—");
  });
});

describe("subscriptionAccess", () => {
  it("trialing and active grant access", () => {
    expect(subscriptionAccess(activeSub(), NOW).access).toBe(true);
    const trial = activeSub({ status: "trialing", trial_end: NOW + 3600 });
    const r = subscriptionAccess(trial, NOW);
    expect(r.access).toBe(true);
    expect(r.banner).toContain("Trial");
  });

  it("free / none / null grant nothing and show no banner", () => {
    const free: CurrentSubscription = {
      org_id: "o",
      tier: "free",
      status: "none",
    };
    expect(subscriptionAccess(free, NOW)).toEqual({
      access: false,
      banner: null,
      tone: null,
    });
    expect(subscriptionAccess(null, NOW).access).toBe(false);
  });

  it("past_due within grace keeps access with countdown", () => {
    const sub = activeSub({
      status: "past_due",
      period_end: NOW - 86400,
    });
    const r = subscriptionAccess(sub, NOW);
    expect(r.access).toBe(true);
    expect(r.tone).toBe("warn");
    expect(r.banner).toContain("past due");
    expect(graceSecondsLeft(sub, NOW)).toBeGreaterThan(0);
  });

  it("past_due after grace expiry loses access", () => {
    const sub = activeSub({
      status: "past_due",
      period_end: NOW - PAST_DUE_GRACE_SECONDS - 60,
    });
    const r = subscriptionAccess(sub, NOW);
    expect(r.access).toBe(false);
    expect(r.tone).toBe("danger");
    expect(graceSecondsLeft(sub, NOW)).toBeLessThanOrEqual(0);
  });

  it("canceled with cancel_at_period_end keeps access until period end", () => {
    const sub = activeSub({
      status: "canceled",
      cancel_at_period_end: true,
      period_end: NOW + 86400,
    });
    const r = subscriptionAccess(sub, NOW);
    expect(r.access).toBe(true);
    expect(r.banner).toContain("access until");
  });

  it("canceled after period end loses access", () => {
    const sub = activeSub({
      status: "canceled",
      cancel_at_period_end: true,
      period_end: NOW - 10,
    });
    expect(subscriptionAccess(sub, NOW).access).toBe(false);
  });

  it("active with cancel_at_period_end keeps access with cancel banner", () => {
    const sub = activeSub({
      cancel_at_period_end: true,
      period_end: NOW + 100,
    });
    const r = subscriptionAccess(sub, NOW);
    expect(r.access).toBe(true);
    expect(r.banner).toContain("Cancels at period end");
  });

  it("active with scheduled_tier shows downgrade notice", () => {
    const sub = activeSub({ scheduled_tier: "pro", tier: "max" });
    const r = subscriptionAccess(sub, NOW);
    expect(r.access).toBe(true);
    expect(r.banner).toContain("pro");
  });

  it("unpaid and expired lose access", () => {
    expect(
      subscriptionAccess(activeSub({ status: "unpaid" }), NOW).access,
    ).toBe(false);
    expect(
      subscriptionAccess(activeSub({ status: "expired" }), NOW).access,
    ).toBe(false);
  });
});

describe("previewChange", () => {
  it("upgrades are immediate", () => {
    expect(previewChange("free", "pro")).toMatchObject({
      kind: "upgrade",
      immediate: true,
    });
    expect(previewChange("pro", "max").immediate).toBe(true);
    expect(previewChange("pro", "team").immediate).toBe(true);
  });

  it("downgrades are end-of-period", () => {
    const r = previewChange("team", "pro");
    expect(r.kind).toBe("downgrade");
    expect(r.immediate).toBe(false);
    expect(r.copy).toContain("period end");
  });

  it("same tier reports no direction", () => {
    expect(previewChange("pro", "pro").kind).toBe("same");
  });
});

describe("validateSeats", () => {
  it("requires a positive integer", () => {
    expect(validateSeats(0, 25).ok).toBe(false);
    expect(validateSeats(-2, 25).ok).toBe(false);
    expect(validateSeats(1.5, 25).ok).toBe(false);
    expect(validateSeats("3", 25).ok).toBe(false);
    expect(validateSeats(Number.NaN, 25).ok).toBe(false);
    expect(validateSeats(1, 25)).toEqual({ ok: true, error: null });
  });

  it("enforces max_seats caps", () => {
    expect(validateSeats(26, 25).ok).toBe(false);
    expect(validateSeats(25, 25).ok).toBe(true);
    expect(validateSeats(26, 25).error).toContain("25");
  });

  it("-1 means unlimited", () => {
    expect(validateSeats(10000, -1).ok).toBe(true);
  });
});

describe("guards + search params + catalog lookup", () => {
  it("isPaidTier / isValidCycle", () => {
    expect(isPaidTier("pro")).toBe(true);
    expect(isPaidTier("free")).toBe(false);
    expect(isPaidTier(null)).toBe(false);
    expect(isValidCycle("annual")).toBe(true);
    expect(isValidCycle("weekly")).toBe(false);
  });

  it("parses ?plan=&cycle= preselect, rejects junk", () => {
    expect(parseBillingSearchParams("?plan=max&cycle=annual")).toEqual({
      plan: "max",
      cycle: "annual",
    });
    expect(parseBillingSearchParams("?plan=bogus&cycle=bogus")).toEqual({
      plan: null,
      cycle: null,
    });
    expect(parseBillingSearchParams("")).toEqual({ plan: null, cycle: null });
  });

  it("planByTier finds each paid tier", () => {
    expect(planByTier(BILLING_PLANS, "pro")?.name).toBe("Pro");
    expect(planByTier(BILLING_PLANS, "team")?.max_seats).toBe(-1);
  });
});

describe("apiPath backend routing", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("dev (no base): keeps /api/* so the vite proxy strips it", () => {
    vi.stubEnv("VITE_BACKEND_URL", "");
    expect(apiPath("/api/v1/plans")).toBe("/api/v1/plans");
    expect(apiPath("/api/v1/subscriptions?org_id=x")).toBe(
      "/api/v1/subscriptions?org_id=x",
    );
  });

  it("direct (base set): strips /api, backend serves /v1/*", () => {
    vi.stubEnv("VITE_BACKEND_URL", "http://127.0.0.1:8080");
    expect(apiPath("/api/v1/plans")).toBe("http://127.0.0.1:8080/v1/plans");
    expect(apiPath("/api/v1/subscriptions/sub_1/cancel")).toBe(
      "http://127.0.0.1:8080/v1/subscriptions/sub_1/cancel",
    );
  });

  it("direct with trailing slash base: no // duplication, no /api", () => {
    vi.stubEnv("VITE_BACKEND_URL", "http://h:8080/");
    expect(apiPath("/api/v1/entitlement?org_id=o")).toBe(
      "http://h:8080/v1/entitlement?org_id=o",
    );
  });

  it("does not strip non-/api prefixes", () => {
    vi.stubEnv("VITE_BACKEND_URL", "http://127.0.0.1:8080");
    expect(apiPath("/v1/plans")).toBe("http://127.0.0.1:8080/v1/plans");
  });
});
