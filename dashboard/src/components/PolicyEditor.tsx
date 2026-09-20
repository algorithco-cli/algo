import * as React from "react";
import { Button } from "./ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card";
import { getPolicyWithFallback, dryRunWithFallback, publishPolicyWithFallback } from "../lib/api";
import type { DryRunResponse } from "../lib/api";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";

const DEFAULT_POLICY_YAML = `# algorithco guard policy — YAML (versioned bundle)
# Bundle is versioned + detached-sig verified by daemon; rollback protection enforced.
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

function yamlToBundle(yaml: string, version: string) {
  // Static stub: signed_bytes is base64(yaml); sig is mock
  return { version, signed_bytes: btoa(yaml), sig: btoa(`sig:${version}`) };
}

export function PolicyEditor(): JSX.Element {
  const qc = useQueryClient();
  const [yaml, setYaml] = React.useState(DEFAULT_POLICY_YAML);
  const [version, setVersion] = React.useState("v0.3.1-draft");
  const [dryResult, setDryResult] = React.useState<DryRunResponse | null>(null);
  const [publishMsg, setPublishMsg] = React.useState<string | null>(null);

  const policyQ = useQuery({
    queryKey: ["policy"],
    queryFn: () => getPolicyWithFallback({}),
  });

  React.useEffect(() => {
    if (policyQ.data?.bundle?.signed_bytes) {
      try {
        const decoded = atob(policyQ.data.bundle.signed_bytes as string);
        if (decoded && decoded.length > 20) setYaml(decoded);
        if (policyQ.data.bundle.version) setVersion(`${policyQ.data.bundle.version}-draft`);
      } catch {
        // keep default
      }
    }
  }, [policyQ.data]);

  const dryRunMut = useMutation({
    mutationFn: async () => {
      const bundle = yamlToBundle(yaml, version);
      // Dry-run vs recent history — use all mock ids as fallback
      const history_ids = Array.from({ length: 8 }, (_, i) => `evt-000${i + 1}`);
      const res = await dryRunWithFallback({ bundle, history_ids, org_id: "org-demo" });
      return res;
    },
    onSuccess: (data) => setDryResult(data),
  });

  const publishMut = useMutation({
    mutationFn: async () => {
      const bundle = yamlToBundle(yaml, version);
      const res = await publishPolicyWithFallback({ bundle, org_id: "org-demo" });
      return res;
    },
    onSuccess: (data) => {
      setPublishMsg(`Published ${data.version} — daemon will pick up on next poll (/v1/policy). Mock: ${data.ok ? "ok" : "pending"}`);
      qc.invalidateQueries({ queryKey: ["policy"] });
      qc.invalidateQueries({ queryKey: ["audit"] });
      qc.invalidateQueries({ queryKey: ["stats"] });
    },
  });

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Policy editor</CardTitle>
          <CardDescription>
            YAML bundle (versioned, detached-sig). Daemon verifies signature + rollback. Errors → ask.
            <br />
            <span style={{ color: "var(--ag-text-muted)" }}>
              Tip: dry-run replays bundle vs redacted history before publish.
            </span>
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">
          <label className="grid gap-1.5">
            <span className="text-xs font-medium" style={{ color: "var(--ag-text-muted)" }}>
              Bundle version
            </span>
            <input
              value={version}
              onChange={(e) => setVersion(e.target.value)}
              className="rounded-md border px-3 py-2 text-sm font-mono"
              style={{ borderColor: "var(--ag-border)", background: "var(--ag-surface)", color: "var(--ag-text)" }}
              placeholder="v0.3.1"
            />
          </label>
          <label className="grid gap-1.5">
            <span className="text-xs font-medium" style={{ color: "var(--ag-text-muted)" }}>
              Policy YAML
            </span>
            <textarea
              value={yaml}
              onChange={(e) => setYaml(e.target.value)}
              rows={18}
              className="rounded-md border p-3 font-mono text-xs leading-5"
              style={{
                borderColor: "var(--ag-border)",
                background: "var(--ag-surface)",
                color: "var(--ag-text)",
              }}
              spellCheck={false}
            />
          </label>
          <div className="flex flex-wrap gap-2">
            <Button onClick={() => dryRunMut.mutate()} disabled={dryRunMut.isPending} variant="outline">
              {dryRunMut.isPending ? "Dry-running…" : "Dry-run vs history"}
            </Button>
            <Button onClick={() => publishMut.mutate()} disabled={publishMut.isPending}>
              {publishMut.isPending ? "Publishing…" : "Publish (mock)"}
            </Button>
            <Button
              variant="ghost"
              onClick={() => {
                setYaml(DEFAULT_POLICY_YAML);
                setDryResult(null);
                setPublishMsg(null);
              }}
            >
              Reset
            </Button>
          </div>
          <p className="text-xs" style={{ color: "var(--ag-text-muted)" }}>
            Publish is mocked in static build — backend verifies <code>sig</code> + monotonic version when live. Local
            fallback shows <code>algo policy --dry-run</code> semantics.
          </p>
        </CardContent>
      </Card>

      <div className="grid gap-6">
        <Card>
          <CardHeader>
            <CardTitle>Dry-run result</CardTitle>
            <CardDescription>Replays bundle vs redacted history (no source field leaves device).</CardDescription>
          </CardHeader>
          <CardContent>
            {dryRunMut.isError && (
              <div className="rounded-md border p-3 text-sm" style={{ borderColor: "var(--ag-deny)", color: "var(--ag-deny)" }}>
                Dry-run failed — fail-safe would resolve to <strong>ask</strong>. Check YAML syntax.
              </div>
            )}
            {!dryResult && !dryRunMut.isPending && !dryRunMut.isError && (
              <div className="rounded-md border border-dashed p-6 text-center text-sm" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
                No dry-run yet. Edit policy → <em>Dry-run vs history</em>.
              </div>
            )}
            {dryRunMut.isPending && (
              <div className="rounded-md border p-3 text-sm" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
                Evaluating…
              </div>
            )}
            {dryResult && (
              <div className="grid gap-3">
                <div className="rounded-md border p-3 font-mono text-xs" style={{ borderColor: "var(--ag-border)", background: "var(--ag-bg)" }}>
                  {dryResult.result}
                </div>
                <div className="grid gap-1">
                  <span className="text-xs font-medium" style={{ color: "var(--ag-text-muted)" }}>
                    Sample decisions ({dryResult.evaluated} evaluated)
                  </span>
                  <ul className="grid gap-1">
                    {dryResult.decisions.map((d, i) => (
                      <li key={i} className="rounded border px-2 py-1 font-mono text-xs" style={{ borderColor: "var(--ag-border)", background: "var(--ag-surface)" }}>
                        {d}
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Publish status</CardTitle>
            <CardDescription>Daemon picks up new bundle via polling /v1/policy (signed + version-gated).</CardDescription>
          </CardHeader>
          <CardContent>
            {publishMsg ? (
              <div className="rounded-md border p-3 text-sm" style={{ borderColor: "var(--ag-allow)", background: "var(--ag-surface)", color: "var(--ag-text)" }}>
                {publishMsg}
              </div>
            ) : (
              <div className="rounded-md border border-dashed p-6 text-center text-sm" style={{ borderColor: "var(--ag-border)", color: "var(--ag-text-muted)" }}>
                No publish yet.
              </div>
            )}
            {publishMut.isError && (
              <p className="mt-2 text-xs" style={{ color: "var(--ag-deny)" }}>
                Publish failed — ask fallback. Check network / sig. Local mock will still report ok in static build.
              </p>
            )}
            <p className="mt-3 text-xs" style={{ color: "var(--ag-text-muted)" }}>
              Static build mock: no real daemon. In prod, <code>algo policy --publish</code> and daemon sync is E2E via
              backend (login→org→policy→daemon→audit→dashboard).
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
