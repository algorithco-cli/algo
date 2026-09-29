import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { HistoryTable } from "../../components/dashboard/HistoryTable";
import { LiveFeed } from "../../components/dashboard/LiveFeed";
import { recordToEntry } from "../../components/dashboard/entry";
import { mockEntries } from "../../components/dashboard/mock";
import { guardKeys, guardOrgId, guardToken, listAudit } from "../../lib/guard";

/** History: live tail + newest-first table (limit 50), mock fallback. */
export default function DashboardHistory(): JSX.Element {
  const token = guardToken();
  const orgId = guardOrgId();
  const q = useQuery({
    queryKey: guardKeys.audit({ org_id: orgId || undefined, limit: 50 }),
    queryFn: () => listAudit({ org_id: orgId || undefined, limit: 50 }, token),
    enabled: !!token,
    retry: false,
  });
  const entries = q.data ? q.data.records.map(recordToEntry) : mockEntries;

  return (
    <section aria-label="History">
      <h1 className="dash-title">History</h1>
      <p className="muted">
        Every decision shows{" "}
        <strong>action + reason + confidence + source + latency</strong> — same
        as <code>algo why</code>.
      </p>
      <div className="dash-stack">
        <LiveFeed />
        <h2 className="dash-h2">Recent decisions</h2>
        <HistoryTable
          data={entries}
          isLoading={q.isPending && !!token}
          demo={!q.data}
        />
        <div className="card">
          <h3 className="dash-card-title">
            Security findings — P4 placeholder
          </h3>
          <p className="muted small">
            Findings (repo-scan + scanner) will appear here when available. See{" "}
            <Link to="/dashboard/stats" className="dash-why-link">
              Stats → Security findings
            </Link>
            . Shown until the backend audit stream provides findings.
          </p>
        </div>
      </div>
    </section>
  );
}
