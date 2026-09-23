import { Link } from "react-router-dom";
import { Counter } from "../components/Counter";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { VerdictDemo } from "../components/VerdictDemo";
import { STATS } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function Features(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/features" />
      <Section
        id="features"
        kicker="Features"
        title="Safety that stays"
        accent="out of the way"
        lede="Four properties, each pinned by tests and CI gates — not claims."
      >
        <div className="bento">
          <div className="card bento-tile" id="safer">
            <h3>Safer — 13 hard-deny rules</h3>
            <p className="muted">
              rm -rf /, mkfs, dd to devices, fork bombs, curl|sh, chmod 777,
              eval+base64, ssh bypass, netcat shells, ransomware extensions.
              Rules outrank models; no profile overrides a deny. Mutants ≥90% on
              the deny list; 32 proves_ask_on_* tests kill allow-on-error.
            </p>
            <VerdictDemo />
          </div>
          <div className="card bento-tile" id="quieter">
            <h3>Quieter — shadow-first</h3>
            <p className="muted">
              Ships in shadow mode: records would-have-blocked N without
              blocking. Flip to enforce only with explicit{" "}
              <code>algo enforce on</code>. Quiet unless it needs you.
            </p>
          </div>
          <div className="card bento-tile" id="auditable">
            <h3>Auditable — algo why</h3>
            <p className="muted">
              Every decision logs action + reason + confidence + source +
              latency to <code>~/.algo/audit.db</code>. Inspect egress with{" "}
              <code>algo log --show-egress</code>.
            </p>
            <Link to="/how">How the pipeline works →</Link>
          </div>
          <div className="card bento-tile" id="integrations">
            <h3>Integrations — hooks, plugins, MCP</h3>
            <p className="muted">
              Claude Code Bash hooks today. Codex and OpenCode adapters are
              Phase 4 on the versioned proto contract, so adapters stay thin.
            </p>
            <Link to="/docs/install">Install for Claude Code →</Link>
          </div>
        </div>
        <div className="stats-band">
          {STATS.map((s) => (
            <div key={s.label} className="stat">
              <span className="stat-value">
                <Counter to={s.value} prefix={s.prefix} suffix={s.suffix} />
              </span>
              <span className="muted small">{s.label}</span>
            </div>
          ))}
        </div>
      </Section>
    </>
  );
}
