import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import * as React from "react";
import { Link } from "react-router-dom";
import { BillingApiError } from "../../lib/billing";
import {
  dryRunPolicy,
  getPolicy,
  guardKeys,
  guardOrgId,
  guardToken,
  listAudit,
  publishPolicy,
} from "../../lib/guard";

const DEFAULT_POLICY_YAML = `# algorithco guard policy — YAML (versioned bundle)
# Deterministic rules outrank models; unknown/error → ask (never allow).
version: v0.3.0
profile: balanced
rules:
  - id: deny-pipe-to-shell
    when: 'tool_kind == "SHELL" && redacted_payload contains "| sh"'
    action: deny
    reason: "pipe to shell blocked by hard-deny rule"
  - id: ask-destructive-rm
    when: 'shell_argv[0] == "rm" && shell_argv contains "-rf"'
    action: ask
    reason: "destructive rm requires confirmation"
  - id: allow-project-writes
    when: 'tool_kind in ["WRITE","EDIT"] && file_path starts_with "./src/"'
    action: allow
    reason: "write within project scope"
thresholds:
  local_model: 0.72
  jev: 0.65
`;

function errMsg(e: unknown): string {
  return e instanceof BillingApiError
    ? e.status === 402
      ? `Subscription required${e.upgrade ? ` — ${e.upgrade.tier} unlocks ${e.upgrade.feature}` : ""}.`
      : e.message
    : "Request failed.";
}

/** Policy: YAML editor + dry-run vs live history + publish (all live). */
export default function DashboardPolicy(): JSX.Element {
  const qc = useQueryClient();
  const token = guardToken();
  const orgId = guardOrgId();
  const [yaml, setYaml] = React.useState(DEFAULT_POLICY_YAML);
  const authed = !!token && !!orgId;

  const policyQ = useQuery({
    queryKey: guardKeys.org(`policy-${orgId}`),
    queryFn: () => getPolicy("latest", orgId, token),
    enabled: authed,
    retry: false,
  });

  React.useEffect(() => {
    const b64 = policyQ.data?.signed_bytes_b64;
    if (!b64) return;
    try {
      const decoded = atob(b64);
      if (decoded.includes("rules:")) setYaml(decoded);
    } catch {
      /* keep default */
    }
  }, [policyQ.data]);

  const dryRunMut = useMutation({
    mutationFn: async () => {
      const history = await listAudit({ org_id: orgId, limit: 50 }, token);
      return dryRunPolicy(
        {
          org_id: orgId,
          bundle: { content: yaml },
          history_ids: history.records.map((r) => r.trace_id),
        },
        token,
      );
    },
  });

  const publishMut = useMutation({
    mutationFn: () => publishPolicy({ org_id: orgId, content: yaml }, token),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["guard"] });
    },
  });

  if (!authed) {
    return (
      <section aria-label="Policy">
        <h1 className="dash-title">Policy</h1>
        <div className="card">
          <p className="muted">
            <span className="badge badge-ask">signed out</span> Policy editing
            needs a session and org. <Link to="/login">Log in</Link>, pick an
            org on the <Link to="/billing">billing</Link> page, then return
            here.
          </p>
        </div>
      </section>
    );
  }

  return (
    <section aria-label="Policy">
      <h1 className="dash-title">Policy</h1>
      <p className="muted">
        Edit YAML, <strong>dry-run vs history</strong> (replays redacted audit),
        then publish. Backend verifies signature + rollback; errors → ask.
      </p>
      <div className="dash-grid-2">
        <div className="card">
          <h2 className="dash-card-title">Policy editor</h2>
          <label className="dash-field">
            <span className="muted small">Policy YAML</span>
            <textarea
              value={yaml}
              onChange={(e) => setYaml(e.target.value)}
              rows={18}
              className="dash-textarea"
              spellCheck={false}
            />
          </label>
          <div className="dash-actions">
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => dryRunMut.mutate()}
              disabled={dryRunMut.isPending}
            >
              {dryRunMut.isPending ? "Dry-running…" : "Dry-run vs history"}
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => publishMut.mutate()}
              disabled={publishMut.isPending}
            >
              {publishMut.isPending ? "Publishing…" : "Publish"}
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => setYaml(DEFAULT_POLICY_YAML)}
            >
              Reset
            </button>
          </div>
          <p className="muted small">
            {policyQ.data
              ? `Loaded server bundle ${policyQ.data.version}. Publish assigns the next version.`
              : "No server bundle yet — publish creates version 1."}{" "}
            Publish needs the dry_run entitlement (Pro+); otherwise 402 with an
            upgrade hint.
          </p>
        </div>

        <div className="dash-stack">
          <div className="card">
            <h2 className="dash-card-title">Dry-run result</h2>
            {dryRunMut.isPending ? (
              <p className="muted" aria-live="polite">
                Evaluating…
              </p>
            ) : dryRunMut.isError ? (
              <p className="dash-err">
                Dry-run failed ({errMsg(dryRunMut.error)}) — fail-safe resolves
                to <strong>ask</strong>.
              </p>
            ) : dryRunMut.data ? (
              <div className="dash-stack-sm">
                <code className="dash-code-block">{dryRunMut.data.result}</code>
                <p className="muted small">
                  Sample decisions ({dryRunMut.data.evaluated} evaluated)
                </p>
                <ul className="dash-list">
                  {dryRunMut.data.decisions.map((d) => (
                    <li key={d} className="dash-code-line">
                      {d}
                    </li>
                  ))}
                </ul>
              </div>
            ) : (
              <p className="muted">
                No dry-run yet. Edit policy → <em>Dry-run vs history</em>.
              </p>
            )}
          </div>

          <div className="card">
            <h2 className="dash-card-title">Publish status</h2>
            {publishMut.isError ? (
              <p className="dash-err">{errMsg(publishMut.error)}</p>
            ) : publishMut.data ? (
              <p>
                Published <code>{publishMut.data.version}</code> — daemon picks
                it up on next poll of <code>/v1/policy</code>.
              </p>
            ) : (
              <p className="muted">No publish yet.</p>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}
