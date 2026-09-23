import { Link } from "react-router-dom";
import { Breadcrumbs } from "../components/Breadcrumbs";
import { PipelineDiagram } from "../components/PipelineDiagram";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { PIPELINE } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function How(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/how" />
      <div className="wrap" style={{ paddingTop: 24 }}>
        <Breadcrumbs
          trail={[{ label: "Home", to: "/" }, { label: "How it works" }]}
        />
      </div>
      <Section
        id="how"
        kicker="How it works"
        title="One pipeline,"
        accent="milliseconds"
        lede="Hook, parse, decide, render — every hop attaches source + latency. Errors resolve to ask, never allow."
      >
        <div className="card pipeline-card">
          <PipelineDiagram />
        </div>
        <div className="pipeline-captions">
          {PIPELINE.map((p) => (
            <div key={p.title} className="card">
              <h3>
                {p.title} <span className="muted small">{p.sub}</span>
              </h3>
              <p className="muted">{p.body}</p>
            </div>
          ))}
        </div>
        <div className="card">
          <h3>Latency budgets (CI-enforced)</h3>
          <table className="cli-table">
            <thead>
              <tr>
                <th>Layer</th>
                <th>p50</th>
                <th>p99</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>
                  <code>L0 / L1 local</code>
                </td>
                <td>&lt;3ms</td>
                <td>&lt;10ms</td>
              </tr>
              <tr>
                <td>
                  <code>L3 Jev (opt-in)</code>
                </td>
                <td>&lt;250ms</td>
                <td>&lt;800ms</td>
              </tr>
            </tbody>
          </table>
          <p className="muted small">
            Hook client starts in ~1ms. Daemon down, timeout, or parse error →
            ask with exit 0. Fingerprint cache TTL 24h.{" "}
            <Link to="/privacy">Privacy modes</Link> ·{" "}
            <Link to="/docs">Docs</Link>
          </p>
        </div>
      </Section>
    </>
  );
}
