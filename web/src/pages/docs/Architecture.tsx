import { DocsArticle } from "../../components/DocsArticle";
import { Seo } from "../../components/Seo";

export default function Architecture(): JSX.Element {
  return (
    <>
      <Seo path="/docs/architecture" />
      <DocsArticle
        eyebrow="Concepts"
        title="Architecture"
        summary="A contracts-first view of the hook, local decision engine, audit trail, and optional provider boundary."
        path="/docs/architecture"
      >
        <section>
          <h2>Decision path</h2>
          <ol className="architecture-steps">
            <li>
              <strong>Adapter</strong>
              <span>Normalizes an agent hook event.</span>
            </li>
            <li>
              <strong>Parse and redact</strong>
              <span>Extracts intent and masks sensitive values.</span>
            </li>
            <li>
              <strong>Policy engine</strong>
              <span>Returns allow, ask, or deny under a latency budget.</span>
            </li>
            <li>
              <strong>Audit</strong>
              <span>
                Records reason, source, confidence, and latency locally.
              </span>
            </li>
          </ol>
        </section>
        <section>
          <h2>Contracts first</h2>
          <p>
            Protocol definitions are the shared contract. Generated Rust and
            TypeScript types must come from an exact tagged protocol version
            rather than handwritten duplicates.
          </p>
        </section>
        <section>
          <h2>Failure behavior</h2>
          <p>
            Unavailable services, malformed input, timeouts, and parse failures
            must prove <code>ask</code>. This is the central invariant across
            adapters and decision tiers.
          </p>
        </section>
      </DocsArticle>
    </>
  );
}
