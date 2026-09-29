import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { useGuardSession } from "../../hooks/useGuardSession";
import {
  BillingApiError,
  fetchEntitlement,
  fetchSubscription,
  formatPeriodDate,
} from "../../lib/billing";
import { backendHealth, getOrg, guardKeys, queryStats } from "../../lib/guard";

function titleCase(value: string): string {
  return value ? value[0].toUpperCase() + value.slice(1) : value;
}

function queryError(error: unknown): string | null {
  if (!error) return null;
  return error instanceof BillingApiError
    ? error.message
    : "Could not load data";
}

/** Account overview: profile, workspace, subscription, and live usage. */
export default function DashboardOverview(): JSX.Element {
  const { token, orgId, profile, logout } = useGuardSession();
  const enabled = token !== "" && orgId !== "";
  const health = useQuery({
    queryKey: guardKeys.health,
    queryFn: backendHealth,
  });
  const org = useQuery({
    queryKey: guardKeys.org(orgId),
    queryFn: () => getOrg(orgId, token),
    enabled,
    retry: false,
  });
  const subscription = useQuery({
    queryKey: ["guard", "subscription", orgId],
    queryFn: () => fetchSubscription(orgId, token),
    enabled,
    retry: false,
  });
  const entitlement = useQuery({
    queryKey: ["guard", "entitlement", orgId],
    queryFn: () => fetchEntitlement(orgId, token),
    enabled,
    retry: false,
  });
  const stats = useQuery({
    queryKey: guardKeys.stats({ org_id: orgId, granularity: "day", limit: 30 }),
    queryFn: () =>
      queryStats({ org_id: orgId, granularity: "day", limit: 30 }, token),
    enabled,
    retry: false,
  });

  const paid =
    subscription.data && subscription.data.tier !== "free"
      ? subscription.data
      : null;
  const plan = subscription.data?.tier ?? entitlement.data?.tier ?? "free";
  const status =
    subscription.data?.status ?? entitlement.data?.status ?? "none";
  const total = stats.data?.total ?? 0;
  const displayName =
    profile?.name ?? profile?.email ?? profile?.subject ?? "Account";
  const dataError =
    queryError(org.error) ??
    queryError(subscription.error) ??
    queryError(entitlement.error) ??
    queryError(stats.error);

  return (
    <section aria-label="Dashboard overview">
      <div className="dash-heading-row">
        <div>
          <h1 className="dash-title">Welcome, {displayName}</h1>
          <p className="muted">
            Your account, protection usage, subscription, and workspace status.
          </p>
        </div>
        <button type="button" className="btn" onClick={logout}>
          Log out
        </button>
      </div>

      {dataError ? <p className="dash-err">{dataError}</p> : null}

      <div className="dash-grid dash-overview-grid">
        <article className="card dash-profile-card">
          <div className="dash-avatar" aria-hidden="true">
            {displayName.slice(0, 1).toUpperCase()}
          </div>
          <div>
            <h2 className="dash-card-title">Profile</h2>
            <p className="dash-card-value">{displayName}</p>
            <p className="muted small">
              {profile?.email ?? profile?.subject ?? "Signed-in account"}
            </p>
            <p className="muted small">
              Signed in with {titleCase(profile?.provider ?? "session")}
            </p>
          </div>
        </article>

        <article className="card">
          <h2 className="dash-card-title">Subscription</h2>
          <p className="dash-card-value">
            {titleCase(plan)}{" "}
            <span className="badge badge-allow">{status}</span>
          </p>
          {paid ? (
            <p className="muted small">
              {paid.seats} {paid.seats === 1 ? "seat" : "seats"} · renews{" "}
              {formatPeriodDate(paid.period_end)}
            </p>
          ) : (
            <p className="muted small">
              No paid subscription for this workspace.
            </p>
          )}
          <Link className="dash-card-link" to="/billing">
            Manage subscription →
          </Link>
        </article>

        <article className="card">
          <h2 className="dash-card-title">Workspace</h2>
          <p className="dash-card-value">
            {org.data?.org.org_name ?? (orgId ? "Loading…" : "Not created")}
          </p>
          <p className="muted small dash-truncate">
            {orgId || "Create a workspace above to enable live usage."}
          </p>
          <span
            className={`badge ${health.isSuccess ? "badge-allow" : "badge-ask"}`}
          >
            backend {health.isSuccess ? "online" : "checking"}
          </span>
        </article>
      </div>

      <div className="dash-grid-4">
        <article className="card">
          <h2 className="dash-card-title">Total decisions</h2>
          <p className="dash-kpi">{total}</p>
          <p className="muted small">Last 30 daily buckets</p>
        </article>
        <article className="card">
          <h2 className="dash-card-title">Allowed</h2>
          <p className="dash-kpi dash-allow">{stats.data?.allow ?? 0}</p>
          <p className="muted small">Low-risk actions approved</p>
        </article>
        <article className="card">
          <h2 className="dash-card-title">Asked</h2>
          <p className="dash-kpi dash-ask">{stats.data?.ask ?? 0}</p>
          <p className="muted small">Actions escalated to you</p>
        </article>
        <article className="card">
          <h2 className="dash-card-title">Denied</h2>
          <p className="dash-kpi dash-deny">{stats.data?.deny ?? 0}</p>
          <p className="muted small">Dangerous actions blocked</p>
        </article>
      </div>

      <div className="dash-grid-2">
        <article className="card">
          <h2 className="dash-card-title">Plan limits</h2>
          <dl className="dash-details">
            <div>
              <dt>Retention</dt>
              <dd>{entitlement.data?.retention_days ?? 0} days</dd>
            </div>
            <div>
              <dt>Rate multiplier</dt>
              <dd>{entitlement.data?.rate_multiplier ?? 1}×</dd>
            </div>
            <div>
              <dt>Max seats</dt>
              <dd>
                {entitlement.data?.max_seats === -1
                  ? "Unlimited"
                  : (entitlement.data?.max_seats ?? 1)}
              </dd>
            </div>
            <div>
              <dt>Average latency</dt>
              <dd>{Math.round(stats.data?.avg_latency_ms ?? 0)} ms</dd>
            </div>
          </dl>
        </article>
        <article className="card">
          <h2 className="dash-card-title">Quick actions</h2>
          <div className="dash-quick-actions">
            <Link className="btn" to="/dashboard/history">
              View history
            </Link>
            <Link className="btn" to="/dashboard/policy">
              Edit policy
            </Link>
            <Link className="btn" to="/dashboard/stats">
              Open usage stats
            </Link>
            <Link className="btn" to="/docs/install">
              Connect an agent
            </Link>
          </div>
        </article>
      </div>
    </section>
  );
}
