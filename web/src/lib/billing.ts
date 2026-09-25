/* web — billing client + pure helpers for the /billing page.
 * Backend REST contract (code against this exact shape):
 *   GET  /v1/plans, POST /v1/subscriptions, GET /v1/subscriptions?org_id=X,
 *   POST /v1/subscriptions/:id/change, POST /v1/subscriptions/:id/cancel,
 *   POST /v1/subscriptions/:id/activate, POST /v1/subscriptions/:id/seats, GET /v1/entitlement?org_id=X.
 * Auth: `Authorization: Bearer <token>` on every route except GET /v1/plans.
 * All calls go through `apiPath()` so dev hits same-origin /api/* (vite proxy
 * → http://127.0.0.1:8080, prefix stripped) and direct builds (VITE_BACKEND_URL
 * set to a bare origin, e.g. http://127.0.0.1:8080) hit /v1/* on that origin.
 * NOTE: `vite preview` serves dist/ with NO proxy — billing needs either
 * `vite dev` or a VITE_BACKEND_URL build + running backend.
 */

export type PaidTier = "pro" | "max" | "team";
export type Tier = "free" | PaidTier;
export type BillingCycle = "monthly" | "annual";
export type SubscriptionStatus =
  | "trialing"
  | "active"
  | "past_due"
  | "canceled"
  | "unpaid"
  | "expired"
  | "none";

export interface PlanFeatures {
  signed_bundles: boolean;
  dry_run: boolean;
  dashboard: boolean;
  per_user_stats: boolean;
  oauth_org: boolean;
  siem_export: boolean;
  sso_scim: boolean;
}

export interface Plan {
  tier: PaidTier;
  name: string;
  monthly_cents: number;
  annual_cents: number;
  seats_included: number;
  /** -1 = unlimited seats. */
  max_seats: number;
  rate_multiplier: number;
  retention_days: number;
  features: PlanFeatures;
}

export interface Subscription {
  id: string;
  org_id: string;
  tier: PaidTier;
  status: Exclude<SubscriptionStatus, "none">;
  seats: number;
  cycle: BillingCycle;
  price_cents: number;
  period_start: number;
  period_end: number;
  trial_end: number | null;
  cancel_at_period_end: boolean;
  scheduled_tier: string | null;
  provider: "manual";
}

export interface FreeSubscription {
  org_id: string;
  tier: "free";
  status: "none";
}

export type CurrentSubscription = Subscription | FreeSubscription;

export interface Entitlement {
  tier: Tier;
  status: string;
  valid: boolean;
  features: Partial<PlanFeatures>;
  limits: {
    max_seats: number;
    rate_multiplier: number;
    retention_days: number;
  };
  subscription: Subscription | null;
  upgrade_hint: string | null;
}

export interface UpgradeHint {
  tier: string;
  feature: string;
}

export interface ApiErrorBody {
  error: string;
  upgrade?: UpgradeHint;
}

/** Past-due grace window: subscription still grants access this long after period_end. */
export const PAST_DUE_GRACE_DAYS = 7;
export const PAST_DUE_GRACE_SECONDS = PAST_DUE_GRACE_DAYS * 24 * 60 * 60;

export const TIER_RANK: Record<Tier, number> = {
  free: 0,
  pro: 1,
  max: 2,
  team: 3,
};

export const PAID_TIERS: readonly PaidTier[] = ["pro", "max", "team"];

/* Static plan catalog — SAME per-seat prices as web/src/pages/Pricing.tsx
 * (pro $12/$10, max $29/$24, team $49/$41 per seat/mo). Used as the picker
 * source and as a fallback when GET /v1/plans is unreachable.
 */
export const BILLING_PLANS: readonly Plan[] = [
  {
    tier: "pro",
    name: "Pro",
    monthly_cents: 1200,
    annual_cents: 1000,
    seats_included: 1,
    max_seats: 25,
    rate_multiplier: 2,
    retention_days: 90,
    features: {
      signed_bundles: true,
      dry_run: true,
      dashboard: true,
      per_user_stats: true,
      oauth_org: false,
      siem_export: false,
      sso_scim: false,
    },
  },
  {
    tier: "max",
    name: "Max",
    monthly_cents: 2900,
    annual_cents: 2400,
    seats_included: 1,
    max_seats: 100,
    rate_multiplier: 5,
    retention_days: 365,
    features: {
      signed_bundles: true,
      dry_run: true,
      dashboard: true,
      per_user_stats: true,
      oauth_org: true,
      siem_export: true,
      sso_scim: false,
    },
  },
  {
    tier: "team",
    name: "Team",
    monthly_cents: 4900,
    annual_cents: 4100,
    seats_included: 1,
    max_seats: -1,
    rate_multiplier: 10,
    retention_days: 730,
    features: {
      signed_bundles: true,
      dry_run: true,
      dashboard: true,
      per_user_stats: true,
      oauth_org: true,
      siem_export: true,
      sso_scim: true,
    },
  },
];

export function isPaidTier(tier: string | null | undefined): tier is PaidTier {
  return tier === "pro" || tier === "max" || tier === "team";
}

export function isValidCycle(
  cycle: string | null | undefined,
): cycle is BillingCycle {
  return cycle === "monthly" || cycle === "annual";
}

export function planByTier(
  plans: readonly Plan[],
  tier: PaidTier,
): Plan | undefined {
  return plans.find((p) => p.tier === tier);
}

/* ---------- price math (all amounts in integer cents) ---------- */

export function monthlyTotalCents(plan: Plan, seats: number): number {
  return plan.monthly_cents * seats;
}

/** Monthly charge when on the annual cycle (per-seat annual rate × seats). */
export function annualMonthlyTotalCents(plan: Plan, seats: number): number {
  return plan.annual_cents * seats;
}

/** Full yearly charge on the annual cycle. */
export function annualYearlyTotalCents(plan: Plan, seats: number): number {
  return plan.annual_cents * 12 * seats;
}

/** Yearly savings of annual vs monthly billing. */
export function yearlySavingsCents(plan: Plan, seats: number): number {
  return (plan.monthly_cents - plan.annual_cents) * 12 * seats;
}

export function formatCentsUsd(cents: number): string {
  const sign = cents < 0 ? "-" : "";
  const abs = Math.abs(cents);
  const dollars = Math.floor(abs / 100);
  const rest = abs % 100;
  return `${sign}$${dollars}${rest === 0 ? "" : `.${String(rest).padStart(2, "0")}`}`;
}

export function formatPeriodDate(epochSeconds: number): string {
  if (!Number.isFinite(epochSeconds) || epochSeconds <= 0) return "—";
  return new Date(epochSeconds * 1000).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/* ---------- status → access + banner ---------- */

export type BannerTone = "info" | "warn" | "danger";

export interface AccessPreview {
  access: boolean;
  banner: string | null;
  tone: BannerTone | null;
}

export function graceSecondsLeft(
  sub: CurrentSubscription,
  nowSec: number,
): number {
  if (sub.tier === "free") return 0;
  if (sub.status !== "past_due") return 0;
  return sub.period_end + PAST_DUE_GRACE_SECONDS - nowSec;
}

export function subscriptionAccess(
  sub: CurrentSubscription | null,
  nowSec: number = Math.floor(Date.now() / 1000),
): AccessPreview {
  if (!sub || sub.tier === "free") {
    return { access: false, banner: null, tone: null };
  }
  const s = sub;
  switch (s.status) {
    case "trialing":
      return {
        access: true,
        banner:
          s.trial_end && s.trial_end > nowSec
            ? `Trial active — trial ends ${formatPeriodDate(s.trial_end)}.`
            : "Trial active.",
        tone: "info",
      };
    case "active":
      if (s.cancel_at_period_end) {
        return {
          access: true,
          banner: `Cancels at period end — access until ${formatPeriodDate(s.period_end)}.`,
          tone: "warn",
        };
      }
      if (s.scheduled_tier) {
        return {
          access: true,
          banner: `Downgrade to ${s.scheduled_tier} scheduled at period end (${formatPeriodDate(s.period_end)}).`,
          tone: "info",
        };
      }
      return { access: true, banner: null, tone: null };
    case "past_due": {
      const left = graceSecondsLeft(s, nowSec);
      if (left > 0) {
        const days = Math.max(1, Math.ceil(left / 86400));
        return {
          access: true,
          banner: `Payment past due — update payment within ${days} day${days === 1 ? "" : "s"} to keep access.`,
          tone: "warn",
        };
      }
      return {
        access: false,
        banner:
          "Payment past due — grace period expired. Access paused until payment succeeds.",
        tone: "danger",
      };
    }
    case "canceled":
      if (s.cancel_at_period_end && nowSec < s.period_end) {
        return {
          access: true,
          banner: `Canceled — access until ${formatPeriodDate(s.period_end)}.`,
          tone: "warn",
        };
      }
      return {
        access: false,
        banner: "Subscription ended. Resubscribe to restore access.",
        tone: "danger",
      };
    case "unpaid":
    case "expired":
      return {
        access: false,
        banner: "Subscription ended. Resubscribe to restore access.",
        tone: "danger",
      };
    default:
      return { access: false, banner: null, tone: null };
  }
}

/* ---------- upgrade / downgrade preview ---------- */

export interface ChangePreview {
  kind: "upgrade" | "downgrade" | "same";
  immediate: boolean;
  copy: string;
}

export function previewChange(current: Tier, target: PaidTier): ChangePreview {
  const from = TIER_RANK[current] ?? 0;
  const to = TIER_RANK[target] ?? 0;
  if (from === to) {
    return {
      kind: "same",
      immediate: false,
      copy: "Same plan — only seats or billing cycle would change.",
    };
  }
  if (to > from) {
    return {
      kind: "upgrade",
      immediate: true,
      copy: `Upgrade to ${target} takes effect immediately.`,
    };
  }
  return {
    kind: "downgrade",
    immediate: false,
    copy: `Downgrade to ${target} is scheduled at period end — current plan stays until then.`,
  };
}

/* ---------- seats validation ---------- */

export interface SeatsValidation {
  ok: boolean;
  error: string | null;
}

export function validateSeats(
  seats: unknown,
  maxSeats: number,
): SeatsValidation {
  if (typeof seats !== "number" || !Number.isInteger(seats)) {
    return { ok: false, error: "Seats must be a whole number." };
  }
  if (seats < 1) {
    return { ok: false, error: "Seats must be at least 1." };
  }
  if (maxSeats !== -1 && seats > maxSeats) {
    return {
      ok: false,
      error: `This plan allows at most ${maxSeats} seats.`,
    };
  }
  return { ok: true, error: null };
}

/* ---------- /billing search params (preselect from Pricing CTAs) ---------- */

export interface BillingPreselect {
  plan: PaidTier | null;
  cycle: BillingCycle | null;
}

export function parseBillingSearchParams(search: string): BillingPreselect {
  const params = new URLSearchParams(
    search.startsWith("?") ? search : `?${search}`,
  );
  const planRaw = params.get("plan");
  const cycleRaw = params.get("cycle");
  return {
    plan: isPaidTier(planRaw) ? planRaw : null,
    cycle: isValidCycle(cycleRaw) ? cycleRaw : null,
  };
}

/* ---------- backend base + typed client ---------- */

export function getBackendBase(): string {
  const env = import.meta.env?.VITE_BACKEND_URL as string | undefined;
  if (typeof env === "string" && env.length > 0) return env.replace(/\/$/, "");
  return "";
}

export function apiPath(path: string): string {
  const p = path.startsWith("/") ? path : `/${path}`;
  const base = getBackendBase();
  // Dev (no base): keep /api/* so the vite proxy strips it (see vite.config.ts).
  // Direct (base set, e.g. VITE_BACKEND_URL=http://127.0.0.1:8080): backend
  // serves /v1/* with no /api prefix — strip it to avoid double-prefix 404s.
  if (base) return `${base}${p.replace(/^\/api(?=\/|$)/, "")}`;
  return p;
}

export class BillingApiError extends Error {
  status: number;
  upgrade: UpgradeHint | null;
  constructor(
    status: number,
    message: string,
    upgrade: UpgradeHint | null = null,
  ) {
    super(message);
    this.name = "BillingApiError";
    this.status = status;
    this.upgrade = upgrade;
  }
}

async function parseError(res: Response): Promise<BillingApiError> {
  let message = `Request failed (${res.status})`;
  let upgrade: UpgradeHint | null = null;
  try {
    const body = (await res.json()) as ApiErrorBody;
    if (body && typeof body.error === "string" && body.error.length > 0) {
      message = body.error;
    }
    if (body?.upgrade && typeof body.upgrade.tier === "string") {
      upgrade = {
        tier: body.upgrade.tier,
        feature:
          typeof body.upgrade.feature === "string" ? body.upgrade.feature : "",
      };
    }
  } catch {
    /* non-JSON error body — keep the default message */
  }
  return new BillingApiError(res.status, message, upgrade);
}

async function apiFetch<T>(
  path: string,
  token: string | null,
  init?: RequestInit,
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (
    init?.headers &&
    typeof init.headers === "object" &&
    !Array.isArray(init.headers)
  ) {
    for (const [k, v] of Object.entries(
      init.headers as Record<string, string>,
    )) {
      headers[k] = v;
    }
  }
  if (token) headers.Authorization = `Bearer ${token}`;
  const url = apiPath(path);
  let res: Response;
  try {
    res = await fetch(url, { ...init, headers });
  } catch {
    throw new BillingApiError(
      0,
      "Backend unreachable (is algo-backend running on http://127.0.0.1:8080?).",
      null,
    );
  }
  if (!res.ok) throw await parseError(res);
  if (res.status === 204) return undefined as T;
  try {
    return (await res.json()) as T;
  } catch {
    throw new BillingApiError(
      res.status,
      `Backend returned non-JSON (expected JSON from ${url}).`,
      null,
    );
  }
}

export function fetchPlans(): Promise<{ plans: Plan[] }> {
  return apiFetch<{ plans: Plan[] }>("/api/v1/plans", null);
}

export function fetchSubscription(
  orgId: string,
  token: string,
): Promise<CurrentSubscription> {
  return apiFetch<CurrentSubscription>(
    `/api/v1/subscriptions?org_id=${encodeURIComponent(orgId)}`,
    token,
  );
}

export function createSubscription(
  args: { org_id: string; tier: PaidTier; cycle: BillingCycle; seats: number },
  token: string,
): Promise<Subscription> {
  return apiFetch<Subscription>("/api/v1/subscriptions", token, {
    method: "POST",
    body: JSON.stringify(args),
  });
}

export function changeSubscription(
  id: string,
  args: { tier?: PaidTier; seats?: number; cycle?: BillingCycle },
  token: string,
): Promise<Subscription> {
  return apiFetch<Subscription>(
    `/api/v1/subscriptions/${encodeURIComponent(id)}/change`,
    token,
    { method: "POST", body: JSON.stringify(args) },
  );
}

export function cancelSubscription(
  id: string,
  token: string,
  atPeriodEnd = true,
): Promise<Subscription> {
  return apiFetch<Subscription>(
    `/api/v1/subscriptions/${encodeURIComponent(id)}/cancel`,
    token,
    { method: "POST", body: JSON.stringify({ at_period_end: atPeriodEnd }) },
  );
}

export function activateSubscription(
  id: string,
  token: string,
): Promise<Subscription> {
  return apiFetch<Subscription>(
    `/api/v1/subscriptions/${encodeURIComponent(id)}/activate`,
    token,
    { method: "POST" },
  );
}

export function updateSeats(
  id: string,
  seats: number,
  token: string,
): Promise<Subscription> {
  return apiFetch<Subscription>(
    `/api/v1/subscriptions/${encodeURIComponent(id)}/seats`,
    token,
    { method: "POST", body: JSON.stringify({ seats }) },
  );
}

export function fetchEntitlement(
  orgId: string,
  token: string,
): Promise<Entitlement> {
  return apiFetch<Entitlement>(
    `/api/v1/entitlement?org_id=${encodeURIComponent(orgId)}`,
    token,
  );
}

/* ---------- localStorage persistence (token + org id; never log values) ---------- */

const TOKEN_KEY = "algo-billing-token";
const ORG_KEY = "algo-billing-org";

export function loadStoredToken(): string {
  try {
    return window.localStorage.getItem(TOKEN_KEY) ?? "";
  } catch {
    return "";
  }
}

export function saveStoredToken(token: string): void {
  try {
    if (token) window.localStorage.setItem(TOKEN_KEY, token);
    else window.localStorage.removeItem(TOKEN_KEY);
  } catch {
    /* storage unavailable — form state still holds the value */
  }
}

export function loadStoredOrgId(): string {
  try {
    return window.localStorage.getItem(ORG_KEY) ?? "";
  } catch {
    return "";
  }
}

export function saveStoredOrgId(orgId: string): void {
  try {
    if (orgId) window.localStorage.setItem(ORG_KEY, orgId);
    else window.localStorage.removeItem(ORG_KEY);
  } catch {
    /* storage unavailable — ignore */
  }
}
