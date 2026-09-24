import { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { useRevealOnMount } from "../hooks/useRevealOnMount";
import {
  BILLING_PLANS,
  BillingApiError,
  type BillingCycle,
  type CurrentSubscription,
  type Entitlement,
  type PaidTier,
  type Plan,
  type Subscription,
  type UpgradeHint,
  activateSubscription,
  annualMonthlyTotalCents,
  annualYearlyTotalCents,
  cancelSubscription,
  changeSubscription,
  createSubscription,
  fetchEntitlement,
  fetchPlans,
  fetchSubscription,
  formatCentsUsd,
  formatPeriodDate,
  loadStoredOrgId,
  loadStoredToken,
  monthlyTotalCents,
  parseBillingSearchParams,
  planByTier,
  previewChange,
  saveStoredOrgId,
  saveStoredToken,
  subscriptionAccess,
  updateSeats,
  validateSeats,
  yearlySavingsCents,
} from "../lib/billing";

type LoadState = "idle" | "loading" | "ready" | "error";

function statusBadgeClass(sub: CurrentSubscription): string {
  if (sub.tier === "free") return "badge";
  if (sub.status === "active" || sub.status === "trialing") {
    if ((sub as Subscription).cancel_at_period_end) return "badge badge-ask";
    return "badge badge-allow";
  }
  if (sub.status === "past_due") return "badge badge-ask";
  return "badge badge-deny";
}

const fieldStyle: React.CSSProperties = {
  display: "grid",
  gap: "0.35rem",
  marginBottom: "0.75rem",
};

const inputStyle: React.CSSProperties = {
  background: "var(--ag-bg-raised)",
  border: "1px solid var(--ag-border)",
  color: "var(--ag-text)",
  borderRadius: "var(--ag-radius-sm)",
  padding: "0.5rem 0.75rem",
  font: "inherit",
  width: "100%",
};

export default function Billing(): JSX.Element {
  useRevealOnMount();
  const [searchParams] = useSearchParams();
  const preselect = useMemo(
    () => parseBillingSearchParams(searchParams.toString()),
    [searchParams],
  );

  const [orgId, setOrgId] = useState<string>(() => loadStoredOrgId());
  const [token, setToken] = useState<string>(() => loadStoredToken());
  const [selectedTier, setSelectedTier] = useState<PaidTier>(
    preselect.plan ?? "pro",
  );
  const [annual, setAnnual] = useState<boolean>(
    preselect.cycle ? preselect.cycle === "annual" : true,
  );
  const [seats, setSeats] = useState<number>(1);
  const [plans, setPlans] = useState<readonly Plan[]>(BILLING_PLANS);
  const [sub, setSub] = useState<CurrentSubscription | null>(null);
  const [entitlement, setEntitlement] = useState<Entitlement | null>(null);
  const [loadState, setLoadState] = useState<LoadState>("idle");
  const [error, setError] = useState<string | null>(null);
  const [upgradeHint, setUpgradeHint] = useState<UpgradeHint | null>(null);
  const [pending, setPending] = useState<string | null>(null);
  const [cancelArmed, setCancelArmed] = useState<boolean>(false);

  const cycle: BillingCycle = annual ? "annual" : "monthly";
  const selectedPlan: Plan | undefined = planByTier(plans, selectedTier);
  const maxSeats = selectedPlan?.max_seats ?? -1;
  const seatsCheck = validateSeats(seats, maxSeats);
  const access = useMemo(
    () => subscriptionAccess(sub, Math.floor(Date.now() / 1000)),
    [sub],
  );
  const changePreview = useMemo(() => {
    if (!sub || sub.tier === "free") return null;
    return previewChange(sub.tier, selectedTier);
  }, [sub, selectedTier]);

  const hasCredentials = orgId.trim().length > 0 && token.trim().length > 0;
  const activeSub: Subscription | null =
    sub && sub.tier !== "free" ? (sub as Subscription) : null;

  const handleApiError = useCallback((err: unknown, fallback: string) => {
    if (err instanceof BillingApiError) {
      setError(err.message);
      setUpgradeHint(
        err.status === 402 || err.status === 403 ? err.upgrade : null,
      );
    } else if (err instanceof Error) {
      setError(`${fallback}: ${err.message}`);
      setUpgradeHint(null);
    } else {
      setError(fallback);
      setUpgradeHint(null);
    }
  }, []);

  const refresh = useCallback(async () => {
    if (!hasCredentials) {
      setError("Enter an org ID and API token to load the subscription.");
      return;
    }
    setLoadState("loading");
    setError(null);
    setUpgradeHint(null);
    saveStoredToken(token.trim());
    saveStoredOrgId(orgId.trim());
    try {
      const [s, e] = await Promise.all([
        fetchSubscription(orgId.trim(), token.trim()),
        // Best-effort: surface auth errors (bad token), swallow the rest.
        fetchEntitlement(orgId.trim(), token.trim()).catch(
          (entErr: unknown) => {
            if (
              entErr instanceof BillingApiError &&
              (entErr.status === 401 || entErr.status === 403)
            )
              throw entErr;
            return null;
          },
        ),
      ]);
      setSub(s);
      setEntitlement(e);
      if (s.tier !== "free" && (s as Subscription).seats >= 1) {
        setSeats((s as Subscription).seats);
      }
      setLoadState("ready");
    } catch (err) {
      setLoadState("error");
      handleApiError(err, "Could not load subscription");
    }
  }, [hasCredentials, orgId, token, handleApiError]);

  useEffect(() => {
    let cancelled = false;
    fetchPlans()
      .then((r) => {
        if (!cancelled && Array.isArray(r.plans) && r.plans.length > 0) {
          setPlans(r.plans);
        }
      })
      .catch(() => {
        /* offline backend — keep the static catalog fallback */
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (preselect.plan) setSelectedTier(preselect.plan);
    if (preselect.cycle) setAnnual(preselect.cycle === "annual");
  }, [preselect]);

  async function runAction(
    name: string,
    fn: () => Promise<CurrentSubscription | Subscription | null>,
  ) {
    if (!hasCredentials) {
      setError("Enter an org ID and API token first.");
      return;
    }
    setPending(name);
    setError(null);
    setUpgradeHint(null);
    try {
      const result = await fn();
      if (result) setSub(result as CurrentSubscription);
      try {
        const e = await fetchEntitlement(orgId.trim(), token.trim());
        setEntitlement(e);
      } catch {
        /* entitlement is best-effort after a successful mutation */
      }
      setLoadState("ready");
      if (name === "cancel") setCancelArmed(false);
    } catch (err) {
      handleApiError(err, `${name} failed`);
    } finally {
      setPending(null);
    }
  }

  function handleSubscribe() {
    if (!seatsCheck.ok) {
      setError(seatsCheck.error);
      return;
    }
    void runAction("subscribe", () =>
      createSubscription(
        { org_id: orgId.trim(), tier: selectedTier, cycle, seats },
        token.trim(),
      ),
    );
  }

  function handleChange() {
    if (!activeSub) return;
    if (!seatsCheck.ok) {
      setError(seatsCheck.error);
      return;
    }
    const patch: { tier?: PaidTier; seats?: number; cycle?: BillingCycle } = {
      seats,
      cycle,
    };
    if (selectedTier !== activeSub.tier) patch.tier = selectedTier;
    void runAction("change", () =>
      changeSubscription(activeSub.id, patch, token.trim()),
    );
  }

  function handleSeats() {
    if (!activeSub) return;
    if (!seatsCheck.ok) {
      setError(seatsCheck.error);
      return;
    }
    void runAction("seats", () =>
      updateSeats(activeSub.id, seats, token.trim()),
    );
  }

  function handleCancel() {
    if (!activeSub) return;
    if (!cancelArmed) {
      setCancelArmed(true);
      return;
    }
    void runAction("cancel", () =>
      cancelSubscription(activeSub.id, token.trim(), true),
    );
  }

  function handleActivate() {
    if (!activeSub) return;
    setCancelArmed(false);
    void runAction("activate", () =>
      activateSubscription(activeSub.id, token.trim()),
    );
  }

  const busy = pending !== null;

  return (
    <>
      <Seo path="/billing" />
      <Section
        id="billing"
        kicker="Billing"
        title="Subscription"
        accent="and seats"
        lede="Pick a plan, set seats, and manage the subscription for your org. Prices match the pricing page, per seat."
      >
        <div className="card" style={{ marginBottom: "1rem", padding: "1rem" }}>
          <h3>Connection</h3>
          <p className="muted small">
            Token is stored only in this browser (localStorage) and sent as a
            Bearer header. It is never logged.
          </p>
          <div style={fieldStyle}>
            <label htmlFor="billing-org">Org ID</label>
            <input
              id="billing-org"
              style={inputStyle}
              value={orgId}
              onChange={(e) => setOrgId(e.target.value)}
              placeholder="org_123"
              autoComplete="off"
            />
          </div>
          <div style={fieldStyle}>
            <label htmlFor="billing-token">API token</label>
            <input
              id="billing-token"
              style={inputStyle}
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="Bearer token"
              autoComplete="off"
            />
          </div>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void refresh()}
            disabled={loadState === "loading" || busy || !hasCredentials}
          >
            {loadState === "loading" ? "Loading…" : "Load subscription"}
          </button>
        </div>

        {access.banner ? (
          <output
            className="muted"
            style={{
              display: "block",
              border: "1px solid var(--ag-border)",
              borderRadius: "var(--ag-radius-sm)",
              padding: "0.6rem 0.9rem",
              marginBottom: "1rem",
            }}
          >
            {access.banner}
          </output>
        ) : null}

        {error ? (
          <div
            role="alert"
            style={{
              border: "1px solid var(--ag-deny)",
              borderRadius: "var(--ag-radius-sm)",
              padding: "0.6rem 0.9rem",
              marginBottom: "1rem",
            }}
          >
            <p style={{ margin: 0 }}>{error}</p>
            {upgradeHint ? (
              <p className="muted small" style={{ marginBottom: 0 }}>
                Requires {upgradeHint.tier}
                {upgradeHint.feature ? ` for ${upgradeHint.feature}` : ""}.{" "}
                <Link to="/pricing">Compare plans</Link>
              </p>
            ) : null}
          </div>
        ) : null}

        {loadState === "loading" ? (
          <output className="muted" style={{ display: "block" }}>
            Loading subscription…
          </output>
        ) : null}

        {sub && sub.tier !== "free" ? (
          <div
            className="card"
            style={{ marginBottom: "1rem", padding: "1rem" }}
          >
            <h3>
              Current subscription{" "}
              <span className={statusBadgeClass(sub)}>
                {(sub as Subscription).status}
              </span>
            </h3>
            <dl>
              <div>
                <dt className="muted small">Plan</dt>
                <dd>{(sub as Subscription).tier}</dd>
              </div>
              <div>
                <dt className="muted small">Seats</dt>
                <dd>{(sub as Subscription).seats}</dd>
              </div>
              <div>
                <dt className="muted small">Cycle</dt>
                <dd>{(sub as Subscription).cycle}</dd>
              </div>
              <div>
                <dt className="muted small">Price</dt>
                <dd>{formatCentsUsd((sub as Subscription).price_cents)}</dd>
              </div>
              <div>
                <dt className="muted small">Period</dt>
                <dd>
                  {formatPeriodDate((sub as Subscription).period_start)} –{" "}
                  {formatPeriodDate((sub as Subscription).period_end)}
                </dd>
              </div>
              {(sub as Subscription).scheduled_tier ? (
                <div>
                  <dt className="muted small">Scheduled</dt>
                  <dd>
                    Downgrade to {(sub as Subscription).scheduled_tier} at
                    period end
                  </dd>
                </div>
              ) : null}
            </dl>
            {entitlement ? (
              <p className="muted small">
                Entitlement: {entitlement.tier} ·{" "}
                {entitlement.valid ? "valid" : "not valid"}
                {entitlement.upgrade_hint
                  ? ` · ${entitlement.upgrade_hint}`
                  : ""}
              </p>
            ) : null}
          </div>
        ) : loadState === "ready" ? (
          <div
            className="card"
            style={{ marginBottom: "1rem", padding: "1rem" }}
          >
            <h3>Free plan</h3>
            <p className="muted">
              This org has no paid subscription. Pick a plan below to subscribe.
            </p>
          </div>
        ) : null}

        <div className="card" style={{ padding: "1rem" }}>
          <h3>Choose plan</h3>
          <fieldset className="billing-toggle" style={{ marginBottom: "1rem" }}>
            <legend className="vdemo-sr">Billing period</legend>
            <button
              type="button"
              data-active={!annual ? "" : undefined}
              aria-pressed={!annual}
              onClick={() => setAnnual(false)}
            >
              Monthly
            </button>
            <button
              type="button"
              data-active={annual ? "" : undefined}
              aria-pressed={annual}
              onClick={() => setAnnual(true)}
            >
              Annual
            </button>
          </fieldset>

          <div
            role="radiogroup"
            aria-label="Plan tier"
            style={{ display: "grid", gap: "0.6rem", marginBottom: "1rem" }}
          >
            {plans.map((p) => {
              const perSeat = annual ? p.annual_cents : p.monthly_cents;
              const total = annual
                ? annualMonthlyTotalCents(p, seats)
                : monthlyTotalCents(p, seats);
              return (
                <label
                  key={p.tier}
                  style={{
                    display: "flex",
                    gap: "0.6rem",
                    alignItems: "flex-start",
                    border: "1px solid var(--ag-border)",
                    borderRadius: "var(--ag-radius-sm)",
                    padding: "0.6rem 0.9rem",
                    cursor: "pointer",
                  }}
                >
                  <input
                    type="radio"
                    name="billing-tier"
                    value={p.tier}
                    checked={selectedTier === p.tier}
                    onChange={() => setSelectedTier(p.tier)}
                  />
                  <span>
                    <strong>{p.name}</strong>{" "}
                    <span className="muted small">
                      {formatCentsUsd(perSeat)}/seat/mo
                      {annual ? ", billed annually" : ", billed monthly"}
                    </span>
                    <br />
                    <span className="muted small">
                      {seats} seat{seats === 1 ? "" : "s"}:{" "}
                      {formatCentsUsd(total)}/mo
                      {annual
                        ? ` (${formatCentsUsd(annualYearlyTotalCents(p, seats))}/yr, save ${formatCentsUsd(yearlySavingsCents(p, seats))})`
                        : ""}
                      {p.max_seats !== -1
                        ? ` · up to ${p.max_seats} seats`
                        : " · unlimited seats"}
                    </span>
                  </span>
                </label>
              );
            })}
          </div>

          <div style={fieldStyle}>
            <label htmlFor="billing-seats">Seats</label>
            <div
              style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}
            >
              <button
                type="button"
                className="btn btn-ghost"
                aria-label="Decrease seats"
                onClick={() => setSeats((s) => Math.max(1, s - 1))}
              >
                −
              </button>
              <input
                id="billing-seats"
                style={{ ...inputStyle, width: "6rem", textAlign: "center" }}
                type="number"
                min={1}
                value={seats}
                onChange={(e) => setSeats(Number(e.target.value))}
              />
              <button
                type="button"
                className="btn btn-ghost"
                aria-label="Increase seats"
                onClick={() => setSeats((s) => s + 1)}
              >
                +
              </button>
            </div>
            {!seatsCheck.ok && seatsCheck.error ? (
              <p role="alert" className="muted small" style={{ margin: 0 }}>
                {seatsCheck.error}
              </p>
            ) : null}
          </div>

          {changePreview && activeSub && changePreview.kind !== "same" ? (
            <output className="muted small" style={{ display: "block" }}>
              {changePreview.copy}
            </output>
          ) : null}

          <div style={{ display: "flex", gap: "0.6rem", flexWrap: "wrap" }}>
            {!activeSub ? (
              <button
                type="button"
                className="btn btn-primary"
                onClick={handleSubscribe}
                disabled={busy || !hasCredentials || !seatsCheck.ok}
              >
                {pending === "subscribe" ? "Subscribing…" : "Subscribe"}
              </button>
            ) : (
              <>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={handleChange}
                  disabled={busy || !hasCredentials || !seatsCheck.ok}
                >
                  {pending === "change" ? "Changing…" : "Change plan"}
                </button>
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={handleSeats}
                  disabled={busy || !hasCredentials || !seatsCheck.ok}
                >
                  {pending === "seats" ? "Updating…" : "Update seats"}
                </button>
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={handleCancel}
                  disabled={busy || !hasCredentials}
                  aria-describedby="cancel-help"
                >
                  {pending === "cancel"
                    ? "Canceling…"
                    : cancelArmed
                      ? "Confirm cancel at period end"
                      : "Cancel at period end"}
                </button>
                {activeSub.cancel_at_period_end ? (
                  <button
                    type="button"
                    className="btn btn-ghost"
                    onClick={handleActivate}
                    disabled={busy || !hasCredentials}
                  >
                    {pending === "activate" ? "Reactivating…" : "Reactivate"}
                  </button>
                ) : null}
              </>
            )}
          </div>
          {activeSub ? (
            <p id="cancel-help" className="muted small">
              {cancelArmed
                ? "Click again to confirm. Access continues until the period ends."
                : "Cancel takes effect at period end; access continues until then."}
            </p>
          ) : null}
          <p className="muted small">
            Paid plans billed per seat via external MoR. See{" "}
            <Link to="/pricing">pricing</Link>.
          </p>
        </div>
      </Section>
    </>
  );
}
