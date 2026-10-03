import { Link } from "react-router-dom";
import { DocsArticle } from "../../components/DocsArticle";
import { Seo } from "../../components/Seo";

export default function PrivacySecurity(): JSX.Element {
  return (
    <>
      <Seo path="/docs/privacy-security" />
      <DocsArticle
        eyebrow="Trust"
        title="Privacy and security"
        summary="Understand the local trust boundary before enabling any optional network feature."
        path="/docs/privacy-security"
      >
        <section>
          <h2>Default boundary</h2>
          <p>
            <code>local-only</code> evaluates locally and keeps decision records
            on the device. No remote model is required for the core guard path.
          </p>
        </section>
        <section>
          <h2>Account and cloud features</h2>
          <p>
            Account mode is the default for cloud and team features, but local
            policy evaluation remains available offline. Web tokens remain in
            memory and native tokens remain in the operating-system credential
            vault. The backend sends only the verified account subject and its
            read-only product credential when resolving an entitlement; it does
            not send command text or local audit content in that lookup.
          </p>
          <p>
            Authentication failures are unauthenticated. Entitlement failures
            use a still-valid cached result or the free tier and never alter a
            policy decision.
          </p>
        </section>
        <section>
          <h2>Before enabling egress</h2>
          <ol>
            <li>
              Choose <code>redacted</code> unless unredacted context is strictly
              required.
            </li>
            <li>
              Inspect the exact intended payload with{" "}
              <code>algo log --show-egress</code>.
            </li>
            <li>
              Review the configured provider’s own retention and training terms.
            </li>
            <li>
              Verify a network failure produces <code>ask</code>.
            </li>
          </ol>
        </section>
        <section>
          <h2>Operational checklist</h2>
          <ul>
            <li>
              Restrict access to <code>~/.algo/</code>.
            </li>
            <li>Rotate exposed credentials immediately.</li>
            <li>Keep binaries and policy bundles signature-verified.</li>
            <li>Report vulnerabilities privately.</li>
          </ul>
          <p>
            See the full <Link to="/privacy">privacy policy</Link> and{" "}
            <Link to="/security">security page</Link>.
          </p>
        </section>
      </DocsArticle>
    </>
  );
}
