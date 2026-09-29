import { DocsArticle } from "../../components/DocsArticle";
import { Seo } from "../../components/Seo";

export default function Troubleshooting(): JSX.Element {
  return (
    <>
      <Seo path="/docs/troubleshooting" />
      <DocsArticle
        eyebrow="Help"
        title="Troubleshooting"
        summary="Diagnose installation and decision issues without weakening the safety boundary."
        path="/docs/troubleshooting"
      >
        <section>
          <h2>Run the health check first</h2>
          <pre>
            <code>{"algo doctor\nalgo status"}</code>
          </pre>
          <p>Keep the complete output, but redact secrets before sharing it.</p>
        </section>
        <section>
          <h2>A command unexpectedly asks</h2>
          <ol>
            <li>
              Run <code>algo why &lt;decision-id&gt;</code>.
            </li>
            <li>Check the matched source and active profile.</li>
            <li>Confirm the daemon, policy bundle, and parser are healthy.</li>
            <li>
              Add only the narrowest local rule if the action is understood.
            </li>
          </ol>
        </section>
        <section>
          <h2>Daemon unavailable</h2>
          <p>
            The client should fail safe to <code>ask</code>. Use{" "}
            <code>algo pause</code> when you intentionally need a one-step
            pause; do not edit generated hook files by hand.
          </p>
        </section>
        <section>
          <h2>Clean removal</h2>
          <pre>
            <code>algo uninstall</code>
          </pre>
          <p>
            After uninstalling, compare the integration files with their
            pre-install state and confirm hooks were removed.
          </p>
        </section>
      </DocsArticle>
    </>
  );
}
