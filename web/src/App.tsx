import * as React from "react";

function ThemeToggle(): JSX.Element {
  const [theme, setTheme] = React.useState<"light" | "dark">(() => {
    const a = typeof document !== "undefined" ? document.documentElement.getAttribute("data-theme") : null;
    if (a === "dark" || a === "light") return a;
    if (typeof window !== "undefined" && window.matchMedia?.("(prefers-color-scheme: dark)").matches) return "dark";
    return "light";
  });
  React.useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);
  return (
    <button onClick={() => setTheme((t) => (t === "light" ? "dark" : "light"))} className="btn btn-ghost" style={{ height: 36, padding: "0 10px", fontSize: 13 }} aria-label="Toggle theme">
      <span aria-hidden>{theme === "light" ? "☾" : "☀"}</span> {theme === "light" ? "Dark" : "Light"}
    </button>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: React.ReactNode }): JSX.Element {
  return (
    <section id={id} style={{ scrollMarginTop: 88 }}>
      <h2 style={{ fontSize: 22, fontWeight: 700, margin: "0 0 8px" }}>{title}</h2>
      <div style={{ display: "grid", gap: 16 }}>{children}</div>
    </section>
  );
}

export default function App(): JSX.Element {
  return (
    <div style={{ minHeight: "100vh" }}>
      {/* Header */}
      <header
        style={{
          position: "sticky",
          top: 0,
          zIndex: 30,
          backdropFilter: "blur(8px)",
          background: "color-mix(in srgb, var(--ag-surface) 86%, transparent)",
          borderBottom: "1px solid var(--ag-border)",
        }}
      >
        <div style={{ maxWidth: 1120, margin: "0 auto", padding: "12px 16px", display: "flex", alignItems: "center", gap: 16, flexWrap: "wrap" }}>
          <a href="#" style={{ display: "flex", alignItems: "center", gap: 10, color: "var(--ag-text)", textDecoration: "none" }}>
            <span style={{ display: "inline-flex", height: 28, width: 28, alignItems: "center", justifyContent: "center", borderRadius: 8, background: "var(--ag-brand)", color: "white", fontWeight: 800 }}>a</span>
            <span style={{ fontWeight: 700, letterSpacing: -0.02 }}>algorithco guard</span>
            <span className="kbd" style={{ fontSize: 11 }}>algo</span>
          </a>
          <nav style={{ display: "flex", gap: 8, flexWrap: "wrap", fontSize: 13 }}>
            <a href="#install" style={{ padding: "6px 10px", borderRadius: 8, border: "1px solid var(--ag-border)", background: "var(--ag-surface)", color: "var(--ag-text)", textDecoration: "none" }}>Install</a>
            <a href="#quickstart" style={{ padding: "6px 10px", borderRadius: 8, border: "1px solid var(--ag-border)", background: "var(--ag-surface)", color: "var(--ag-text)", textDecoration: "none" }}>Quickstart</a>
            <a href="#privacy" style={{ padding: "6px 10px", borderRadius: 8, border: "1px solid var(--ag-border)", background: "var(--ag-surface)", color: "var(--ag-text)", textDecoration: "none" }}>Privacy</a>
            <a href="#dataflow" style={{ padding: "6px 10px", borderRadius: 8, border: "1px solid var(--ag-border)", background: "var(--ag-surface)", color: "var(--ag-text)", textDecoration: "none" }}>Dataflow</a>
          </nav>
          <div style={{ marginLeft: "auto", display: "flex", gap: 8, alignItems: "center" }}>
            <a href="#privacy-notice" className="muted" style={{ fontSize: 12, textDecoration: "underline", textUnderlineOffset: 3 }}>
              Privacy notice
            </a>
            <ThemeToggle />
          </div>
        </div>
      </header>

      <main style={{ maxWidth: 1120, margin: "0 auto", padding: "24px 16px 48px", display: "grid", gap: 40 }}>
        {/* Hero */}
        <div className="card" style={{ padding: 24, display: "grid", gap: 16 }}>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
            <span className="badge badge-allow">allow</span>
            <span className="badge badge-ask">ask</span>
            <span className="badge badge-deny">deny</span>
            <span className="muted" style={{ fontSize: 12, alignSelf: "center" }}>
              Decision colors are exclusive — never reused for branding (Variant 1)
            </span>
          </div>
          <h1 style={{ fontSize: 34, fontWeight: 800, letterSpacing: -0.03, lineHeight: 1.15, margin: 0 }}>
            Intelligent control layer for CLI coding agents.
          </h1>
          <p className="muted" style={{ fontSize: 15, margin: 0, maxWidth: 70 + "ch" }}>
            Attaches via hooks / plugins / MCP. Makes agents <strong style={{ color: "var(--ag-text)" }}>safer, quieter, and auditable</strong>. CLI <code>algo</code> · home <code>~/.algo/</code> · socket <code>~/.algo/algo.sock</code> · DB <code>~/.algo/audit.db</code>
          </p>
          <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
            <a href="#install" className="btn btn-primary">Install — algo init</a>
            <a href="https://github.com/anomalyco/opencode" className="btn btn-ghost">GitHub</a>
            <a href="#privacy" className="btn btn-ghost">Privacy &amp; dataflow</a>
          </div>
          <p className="muted" style={{ fontSize: 12, margin: 0 }}>
            Install target ~30s · shadow-first + <code>would-have N</code> digest ·{" "}
            <code>algo why = action+reason+confidence+source+latency</code> · profiles strict/balanced/fast visible · one-step{" "}
            <code>algo pause</code> / <code>algo uninstall</code> even if daemon is broken.
          </p>
        </div>

        {/* Install */}
        <Section id="install" title="Install">
          <div className="grid2">
            <div className="card">
              <h3 style={{ margin: "0 0 6px", fontSize: 15, fontWeight: 700 }}>One-line install</h3>
              <pre>
                <code>{`# macOS / Linux (Windows: see Phase 4 matrix)
curl -fsSL https://algorithco.guard/install.sh | sh
# then
algo init   # ~30s: detect agents → show diff → per-agent consent → backup + additive hook install → privacy prompt → algo doctor
algo doctor # verify install, daemon, socket, DB, agent configs
algo status # counts (allowed / asked / blocked), savings, profile + threshold source`}</code>
              </pre>
              <p className="muted" style={{ fontSize: 12 }}>
                Monorepo stays private until release · <code>algo init</code> is additive (backs up agent configs) and reversible via{" "}
                <code>algo uninstall</code>.
              </p>
            </div>
            <div className="card">
              <h3 style={{ margin: "0 0 6px", fontSize: 15, fontWeight: 700 }}>CLI reference</h3>
              <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
                <thead>
                  <tr style={{ textAlign: "left", borderBottom: "1px solid var(--ag-border)" }}>
                    <th style={{ padding: "8px 6px" }}>Command</th>
                    <th style={{ padding: "8px 6px" }}>Purpose</th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    ["algo init", "Detect agents → diff → consent → hook install → privacy prompt → doctor (~30s)"],
                    ["algo doctor", "Verify install, daemon, socket, DB, agent configs"],
                    ["algo status", "Counts (allowed/asked/blocked), savings, profile + threshold source"],
                    ["algo why", "Explain last decision: action+reason+confidence+source+latency"],
                    ["algo log", "Local audit log. --show-egress inspects exactly what would leave"],
                    ["algo enforce [on|off]", "Shadow → enforce switch (explicit opt-in after shadow trust)"],
                    ["algo pause / resume", "One-step stop/resume. Works even if daemon is broken"],
                    ["algo login", "OAuth device flow for team/cloud (Phase 3)"],
                    ["algo policy", "View / dry-run policy bundles"],
                  ].map(([cmd, desc]) => (
                    <tr key={cmd} style={{ borderBottom: "1px solid var(--ag-border)" }}>
                      <td style={{ padding: "8px 6px" }}>
                        <code>{cmd}</code>
                      </td>
                      <td style={{ padding: "8px 6px" }} className="muted">
                        {desc}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </Section>

        {/* Quickstart */}
        <Section id="quickstart" title="Quickstart (shadow-first)">
          <div className="card">
            <ol style={{ margin: 0, paddingLeft: 18, display: "grid", gap: 8, fontSize: 14 }}>
              <li>
                <code>algo init</code> — approve per-agent hook diff, choose privacy mode (<code>local-only</code> default).
              </li>
              <li>
                Work normally — guard runs in <strong>shadow</strong> and records <code>would-have N</code> digest (no blocking yet).
              </li>
              <li>
                <code>algo status</code> — check counts + <code>algo why</code> explains every decision (action + reason + confidence + source + latency).
              </li>
              <li>
                <code>algo enforce on</code> — after you trust the shadow digest, switch to enforce.
              </li>
              <li>
                <code>algo pause</code> / <code>algo uninstall</code> — one step, works even if daemon is broken.
              </li>
            </ol>
          </div>
        </Section>

        {/* Privacy — must match docs/privacy-dataflow.md verbatim semantics */}
        <Section id="privacy" title="Privacy — how your data is handled">
          <div className="card" style={{ borderLeft: "4px solid var(--ag-brand)" }}>
            <p style={{ margin: 0, fontSize: 13 }} className="muted">
              This text MUST match behavior (source: <code>docs/privacy-dataflow.md</code> owner-confirmed 2026-09-20). No vendor claims without a measurement
              link. No AUP exists (legal index lists only DPA, MCA, Privacy Policy; <code>typesafe.ai/legal/aup</code> 404).
            </p>
          </div>

          <div className="card">
            <h3 style={{ margin: "0 0 8px", fontSize: 15, fontWeight: 700 }}>Modes — BYOK only, Jev off by default</h3>
            <div style={{ overflowX: "auto" }}>
              <table style={{ width: "100%", fontSize: 13, borderCollapse: "collapse" }}>
                <thead>
                  <tr style={{ textAlign: "left", borderBottom: "1px solid var(--ag-border)" }}>
                    <th style={{ padding: "8px 6px" }}>Mode</th>
                    <th style={{ padding: "8px 6px" }}>Behavior</th>
                    <th style={{ padding: "8px 6px" }}>Jev?</th>
                  </tr>
                </thead>
                <tbody>
                  <tr style={{ borderBottom: "1px solid var(--ag-border)" }}>
                    <td style={{ padding: "10px 6px" }}>
                      <code>local-only</code> <span className="badge" style={{ background: "var(--ag-bg)", border: "1px solid var(--ag-border)", fontSize: 10 }}>default</span>
                    </td>
                    <td style={{ padding: "10px 6px" }}>No network. No Jev call. Cloud sync off.</td>
                    <td style={{ padding: "10px 6px" }}>
                      <strong>Off</strong> — no data leaves the machine
                    </td>
                  </tr>
                  <tr style={{ borderBottom: "1px solid var(--ag-border)" }}>
                    <td style={{ padding: "10px 6px" }}>
                      <code>redacted</code> (BYOK, explicit opt-in)
                    </td>
                    <td style={{ padding: "10px 6px" }}>
                      Secrets masked before anything leaves. <strong>Only real-data mode.</strong> Requires <code>ALGO_JEV_API_KEY</code> (BYOK, env-only) + consent.
                    </td>
                    <td style={{ padding: "10px 6px" }}>
                      On — <strong>redacted</strong> <code>canonical.redacted_payload</code> only, to <strong>US</strong>
                    </td>
                  </tr>
                  <tr>
                    <td style={{ padding: "10px 6px" }}>
                      <code>full</code> (BYOK, explicit opt-in)
                    </td>
                    <td style={{ padding: "10px 6px" }}>Unredacted payloads allowed — requires second, clear consent + inspect step.</td>
                    <td style={{ padding: "10px 6px" }}>
                      On — <strong>unredacted</strong> (only with <code>full</code> consent)
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p className="muted" style={{ fontSize: 12, margin: "10px 0 0" }}>
              Defaults: fresh <code>algo init</code> is <code>local-only</code>. BYOK is the <strong>only</strong> real-data mode. No embedded key in binaries (MCA §2.4).
              Proxy mode through our servers stays blocked until legal review (<code>docs/adr/0004-byok-vs-proxy.md</code>).
            </p>
          </div>

          <div className="card">
            <h3 style={{ margin: "0 0 8px", fontSize: 15, fontWeight: 700 }}>Where your data goes (for the consent text)</h3>
            <ul style={{ margin: 0, paddingLeft: 18, fontSize: 13, display: "grid", gap: 6 }}>
              <li>
                <strong>Who:</strong> TypeSafe AI, Inc. and its service providers (subprocessors — all USA): <strong>AWS</strong> (stores) /{" "}
                <strong>Modal, Nebius, CoreWeave</strong> (process, do not store) / <strong>Slack, Google Workspace</strong> (support).
              </li>
              <li>
                <strong>Where:</strong> <strong>US infrastructure</strong> (Privacy Policy “International Visitors”: Services are <strong>hosted in the US</strong>). If you use the
                Services from the EEA/UK or other non-US regions, you <strong>transfer personal data to the US</strong> for storage and processing.
              </li>
              <li>
                <strong>How long:</strong> <strong>unspecified</strong> — “as long as reasonably necessary” (Privacy Policy “Retention”; DPA Schedule I §8 “as long as
                necessary” per purpose + statute of limitations). No fixed retention period, no deletion SLA is published.
              </li>
              <li>
                <strong>Training:</strong> TypeSafe states it <strong>will not train or fine-tune AI/ML models on your prompts or other Input</strong> (Privacy Policy
                “Services”) and <strong>will not disclose Input to a third party other than our service providers</strong> (same section). MCA §4.1/§4.3 separately grants a
                perpetual telemetry license on the other telemetry (hashes/stats/learnings), with no weight training without your prior consent (withholdable).
              </li>
              <li>
                <strong>Zero-retention:</strong> ZDR is <strong>enterprise-only via <code>privacy@typesafe.ai</code></strong> — no ZDR on standard plans.
              </li>
            </ul>
          </div>

          <div className="card">
            <h3 style={{ margin: "0 0 8px", fontSize: 15, fontWeight: 700 }}>Consent text (for <code>algo init</code> privacy prompt — BYOK + <code>redacted</code>)</h3>
            <blockquote style={{ margin: 0, padding: "12px 16px", borderLeft: "3px solid var(--ag-brand)", background: "var(--ag-bg)", borderRadius: 8, fontSize: 13 }}>
              <strong>How Jev works when you turn it on.</strong> <code>local-only</code> (the default) never sends anything to Jev. If you choose{" "}
              <code>redacted</code> (BYOK), every command you approve for Jev is <strong>redacted on your machine</strong> and then sent to{" "}
              <strong>TypeSafe AI’s US infrastructure</strong> (<code>api.typesafe.ai</code>) under your own Jev API key. You set the key as{" "}
              <code>ALGO_JEV_API_KEY</code> (BYOK, env-only, never embedded). TypeSafe says it <strong>will not train on your Input</strong> and{" "}
              <strong>will not disclose Input except to its US service providers</strong> listed above. It keeps your data <strong>as long as reasonably necessary</strong>{" "}
              (no fixed deletion date). If you are outside the US, your data is <strong>transferred to the US</strong>. You can inspect exactly what would leave with{" "}
              <code>algo log --show-egress</code>. Switching back to <code>local-only</code> stops all sending.
            </blockquote>
          </div>

          <div id="privacy-notice" className="card" style={{ borderLeft: "4px solid var(--ag-ask)" }}>
            <h3 style={{ margin: "0 0 8px", fontSize: 15, fontWeight: 700 }}>Privacy notice (for web / docs footers)</h3>
            <p style={{ margin: 0, fontSize: 13, lineHeight: 1.7 }}>
              “When Jev is enabled (BYOK, <code>redacted</code> or <code>full</code>), your redacted (or with <code>full</code>, unredacted) commands are sent to
              TypeSafe AI’s US infrastructure under your own key, processed by its US subprocessors (AWS / Modal / Nebius / CoreWeave), retained as long as
              reasonably necessary (no fixed SLA is published), and covered by TypeSafe’s Privacy Policy statement that it will not train on Input and will not
              disclose Input except to service providers. Non-US users transfer data to the US. <code>local-only</code> sends nothing. ZDR is available only via
              enterprise <code>privacy@typesafe.ai</code>.”
            </p>
          </div>

          <div className="card">
            <h3 style={{ margin: "0 0 8px", fontSize: 15, fontWeight: 700 }}>Rules</h3>
            <ul style={{ margin: 0, paddingLeft: 18, fontSize: 13, display: "grid", gap: 4 }}>
              <li>Redact-before-network, always (one code path for send and <code>--show-egress</code>).</li>
              <li>
                Inspect-what-would-send: <code>algo log --show-egress</code> shows the <strong>exact</strong> redacted outbound payload.
              </li>
              <li>Telemetry is opt-in and never contains source code.</li>
              <li>Datasets are redacted. No real secrets. No user code without consent.</li>
              <li>
                AUP does <strong>not</strong> exist (owner-confirmed 2026-09-20; <code>typesafe.ai/legal/aup</code> 404) — written confirmation requested,{" "}
                <strong>not a blocker</strong>. MCA §2.3(l) reference is dangling.
              </li>
              <li>
                Consent records <code>consent_id</code> + timestamp + scope (<code>redacted</code> vs <code>full</code>) + BYOK key holder; stored outside the dataset.
              </li>
            </ul>
          </div>
        </Section>

        {/* Dataflow */}
        <Section id="dataflow" title="Dataflow">
          <div className="card">
            <pre>
              <code>{`agent event -> adapter (parse) -> redact -> L0/L1/L2 (local, no network)
  -> [L3 Jev — only if mode != local-only AND BYOK key present AND consent is redacted/full:
       redacted only, hosted US, retention "as long as reasonably necessary",
       non-US -> US transfer, timeout -> ask]
  -> decision + reason -> local audit (SQLite ~/.algo/audit.db, redacted_command only)
  -> [opt-in backend sync: redacted only]`}</code>
            </pre>
            <p className="muted" style={{ fontSize: 12 }}>
              Fail-safe: every new I/O / timeout / parse path proves <code>ask</code> downstream (never <code>allow</code>). Deterministic rules outrank models.
              Latency budgets L0/L1 &lt;3/&lt;10ms, L2 &lt;10/&lt;25ms, L3 &lt;250/&lt;800ms — over-budget = no merge or ADR.
            </p>
          </div>
        </Section>

        {/* Dashboard link */}
        <div className="card" style={{ display: "flex", gap: 12, alignItems: "center", flexWrap: "wrap", justifyContent: "space-between" }}>
          <div>
            <div style={{ fontWeight: 700 }}>Team dashboard (Phase 3)</div>
            <div className="muted" style={{ fontSize: 13 }}>
              History with explanations (<code>algo why</code>), blocked/asked breakdown, policy editor + dry-run, per-user/project, SSE live feed. Static SPA, no Node in prod.
            </div>
          </div>
          <a href="http://localhost:5173" className="btn btn-primary" target="_blank" rel="noreferrer">
            Open dashboard →
          </a>
        </div>
      </main>

      <footer style={{ borderTop: "1px solid var(--ag-border)", padding: "18px 16px", background: "var(--ag-surface)" }}>
        <div style={{ maxWidth: 1120, margin: "0 auto", fontSize: 12 }} className="muted">
          <p style={{ margin: 0 }}>
            <strong style={{ color: "var(--ag-text)" }}>algorithco guard</strong> · CLI <code>algo</code> · Tokens Variant 1 single source{" "}
            <code>plans/design-tokens.md</code> → <code>design-tokens.css</code>. Colors: <span style={{ color: "var(--ag-allow)" }}>allow</span> /{" "}
            <span style={{ color: "var(--ag-ask)" }}>ask</span> / <span style={{ color: "var(--ag-deny)" }}>deny</span> only for decisions.
          </p>
          <p style={{ margin: "6px 0 0" }}>
            When Jev is enabled (BYOK, <code>redacted</code> or <code>full</code>), your redacted (or with <code>full</code>, unredacted) commands are sent to TypeSafe AI’s
            US infrastructure under your own key, processed by its US subprocessors (AWS / Modal / Nebius / CoreWeave), retained as long as reasonably necessary (no
            fixed SLA is published), and covered by TypeSafe’s Privacy Policy statement that it will not train on Input and will not disclose Input except to
            service providers. Non-US users transfer data to the US. <code>local-only</code> sends nothing. ZDR is available only via enterprise{" "}
            <code>privacy@typesafe.ai</code>.
          </p>
        </div>
      </footer>
    </div>
  );
}
