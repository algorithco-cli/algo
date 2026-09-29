import { DocsArticle } from "../../components/DocsArticle";
import { Seo } from "../../components/Seo";

export default function Configuration(): JSX.Element {
  return (
    <>
      <Seo path="/docs/configuration" />
      <DocsArticle
        eyebrow="Reference"
        title="Configuration"
        summary="Choose a profile, keep privacy local by default, and make exceptions narrow and reviewable."
        path="/docs/configuration"
      >
        <section>
          <h2>Configuration principles</h2>
          <div className="docs-callout">
            <strong>Start in shadow mode.</strong>
            <span>
              Review the would-have-blocked digest before enforcing a profile.
            </span>
          </div>
          <ul>
            <li>
              Use <strong>balanced</strong> as the general starting profile.
            </li>
            <li>
              Keep hard-deny rules immutable from local convenience exceptions.
            </li>
            <li>Prefer one scoped rule over a broad permanent allow.</li>
            <li>
              Keep credentials in environment variables, never configuration
              files.
            </li>
          </ul>
        </section>
        <section>
          <h2>Profiles</h2>
          <div className="docs-card-grid">
            <div className="docs-mini-card">
              <h3>strict</h3>
              <p>More prompts and the smallest risk envelope.</p>
            </div>
            <div className="docs-mini-card">
              <h3>balanced</h3>
              <p>Default protection for day-to-day development.</p>
            </div>
            <div className="docs-mini-card">
              <h3>fast</h3>
              <p>Fewer prompts, without bypassing hard-deny.</p>
            </div>
          </div>
        </section>
        <section>
          <h2>Verify every change</h2>
          <pre>
            <code>{"algo doctor\nalgo status\nalgo why <decision-id>"}</code>
          </pre>
          <p>
            <code>algo why</code> should show the action, reason, confidence,
            source, and latency for a decision.
          </p>
        </section>
      </DocsArticle>
    </>
  );
}
