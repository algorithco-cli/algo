import { Link } from "react-router-dom";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function Privacy(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/privacy" />
      <div className="wrap" style={{ paddingTop: 24 }}>
        <Breadcrumbs
          trail={[{ label: "Home", to: "/" }, { label: "Privacy" }]}
        />
      </div>
      <Section
        id="privacy"
        kicker="Privacy"
        title="Your data, stated plainly"
        lede="This text matches behavior (source: docs/privacy-dataflow.md, owner-confirmed 2026-09-20). Last reviewed 2026-09-20."
      >
        <div className="card">
          <h3>Modes — BYOK only, Jev off by default</h3>
          <div className="table-scroll">
            <table className="cli-table">
              <thead>
                <tr>
                  <th>Mode</th>
                  <th>Behavior</th>
                  <th>Jev?</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>
                    <code>local-only</code>{" "}
                    <span className="badge badge-allow">default</span>
                  </td>
                  <td>No network. No Jev call. Cloud sync off.</td>
                  <td>
                    <strong>Off</strong> — nothing leaves the machine
                  </td>
                </tr>
                <tr>
                  <td>
                    <code>redacted</code> (BYOK, opt-in)
                  </td>
                  <td>
                    Secrets masked on-machine. Only real-data mode. Requires{" "}
                    <code>ALGO_JEV_API_KEY</code> (env-only) + consent.
                  </td>
                  <td>
                    On — <strong>redacted</strong> payload only, to{" "}
                    <strong>US</strong>
                  </td>
                </tr>
                <tr>
                  <td>
                    <code>full</code> (BYOK, opt-in)
                  </td>
                  <td>
                    Unredacted payloads — second, clear consent + inspect step.
                  </td>
                  <td>
                    On — <strong>unredacted</strong> (only with{" "}
                    <code>full</code> consent)
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
        <div className="card">
          <h3>Where your data goes</h3>
          <ul className="tight-list">
            <li>
              <strong>Who:</strong> TypeSafe AI, Inc. + US subprocessors:{" "}
              <strong>AWS</strong> (stores) /{" "}
              <strong>Modal, Nebius, CoreWeave</strong> (process) /{" "}
              <strong>Slack, Google Workspace</strong> (support).
            </li>
            <li>
              <strong>Where:</strong> <strong>US infrastructure</strong> —
              non-US users transfer data to the US.
            </li>
            <li>
              <strong>How long:</strong> <strong>unspecified</strong> — “as long
              as reasonably necessary”. No fixed deletion SLA is published.
            </li>
            <li>
              <strong>Training:</strong> TypeSafe states it{" "}
              <strong>will not train on your Input</strong> nor disclose it
              beyond service providers.
            </li>
            <li>
              <strong>Zero-retention:</strong> enterprise-only via{" "}
              <code>privacy@typesafe.ai</code>.
            </li>
          </ul>
          <p className="muted small">
            Rules: redact-before-network · inspect exact payload with{" "}
            <code>algo log --show-egress</code> · telemetry opt-in, never code.{" "}
            <code>local-only</code> sends nothing. Back to{" "}
            <Link to="/docs">docs</Link>.
          </p>
        </div>
      </Section>
    </>
  );
}
