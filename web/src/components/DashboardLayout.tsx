import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { NavLink, Navigate, Outlet } from "react-router-dom";
import { useGuardSession } from "../hooks/useGuardSession";
import { BillingApiError } from "../lib/billing";
import { createOrg, guardKeys, listOrgs } from "../lib/guard";

const TABS: readonly { label: string; to: string; end?: boolean }[] = [
  { label: "Overview", to: "/dashboard", end: true },
  { label: "History", to: "/dashboard/history" },
  { label: "Audit", to: "/dashboard/audit" },
  { label: "Policy", to: "/dashboard/policy" },
  { label: "Stats", to: "/dashboard/stats" },
];

/** Dashboard shell: max-width console column + tab nav. Dark-only tokens. */
export function DashboardLayout(): JSX.Element {
  const { token, orgId, signedIn, selectOrg } = useGuardSession();
  const [orgName, setOrgName] = React.useState("My workspace");
  const qc = useQueryClient();
  const orgs = useQuery({
    queryKey: guardKeys.orgs,
    queryFn: () => listOrgs(token),
    enabled: signedIn,
    retry: false,
  });
  const create = useMutation({
    mutationFn: () => createOrg(orgName.trim(), token),
    onSuccess: (org) => {
      selectOrg(org.org_id);
      void qc.invalidateQueries({ queryKey: guardKeys.orgs });
    },
  });

  React.useEffect(() => {
    if (!orgId && orgs.data?.orgs[0]) selectOrg(orgs.data.orgs[0].org_id);
  }, [orgId, orgs.data, selectOrg]);

  if (!signedIn) return <Navigate to="/login" replace />;

  const error =
    orgs.error instanceof BillingApiError ? orgs.error.message : null;
  return (
    <div className="wrap dash">
      <div className="dash-account-bar">
        <div>
          <span className="dash-account-label">Workspace</span>
          {orgs.data?.orgs.length ? (
            <select
              className="dash-select"
              value={orgId}
              onChange={(event) => selectOrg(event.target.value)}
              aria-label="Active workspace"
            >
              {orgs.data.orgs.map((org) => (
                <option key={org.org_id} value={org.org_id}>
                  {org.org_name}
                </option>
              ))}
            </select>
          ) : orgs.isPending ? (
            <span className="muted small">Loading…</span>
          ) : (
            <form
              className="dash-create-org"
              onSubmit={(event) => {
                event.preventDefault();
                if (orgName.trim()) create.mutate();
              }}
            >
              <input
                value={orgName}
                onChange={(event) => setOrgName(event.target.value)}
                maxLength={128}
                aria-label="Workspace name"
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={create.isPending || !orgName.trim()}
              >
                {create.isPending ? "Creating…" : "Create workspace"}
              </button>
            </form>
          )}
        </div>
        {error ? <span className="dash-inline-error">{error}</span> : null}
      </div>
      <nav className="dash-tabs" aria-label="Dashboard">
        {TABS.map(({ label, to, end }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            className={({ isActive }) =>
              isActive ? "dash-tab active" : "dash-tab"
            }
          >
            {label}
          </NavLink>
        ))}
      </nav>
      <Outlet />
    </div>
  );
}
