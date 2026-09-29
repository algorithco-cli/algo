import { useQuery } from "@tanstack/react-query";
import { HistoryTable } from "../../components/dashboard/HistoryTable";
import { recordToEntry } from "../../components/dashboard/entry";
import { mockEntries } from "../../components/dashboard/mock";
import { guardKeys, guardOrgId, guardToken, listAudit } from "../../lib/guard";

/** Audit log: full table (limit 200), mock fallback. */
export default function DashboardAudit(): JSX.Element {
  const token = guardToken();
  const orgId = guardOrgId();
  const q = useQuery({
    queryKey: guardKeys.audit({ org_id: orgId || undefined, limit: 200 }),
    queryFn: () => listAudit({ org_id: orgId || undefined, limit: 200 }, token),
    enabled: !!token,
    retry: false,
  });
  const entries = q.data ? q.data.records.map(recordToEntry) : mockEntries;

  return (
    <section aria-label="Audit log">
      <h1 className="dash-title">Audit log</h1>
      <p className="muted">
        Redacted payloads only — inspect egress with{" "}
        <code>algo log --show-egress</code>.
      </p>
      <div className="dash-stack">
        <HistoryTable
          data={entries}
          isLoading={q.isPending && !!token}
          demo={!q.data}
        />
      </div>
    </section>
  );
}
