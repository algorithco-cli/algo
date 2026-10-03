import { Link } from "react-router-dom";
import { LegalLayout } from "../components/LegalLayout";
import { Seo } from "../components/Seo";

export default function Privacy(): JSX.Element {
  return (
    <>
      <Seo path="/privacy" />
      <LegalLayout
        eyebrow="Privacy"
        title="Privacy policy"
        summary="How algorithco guard handles command context, audit data, website activity, and optional network features."
        reviewed="October 3, 2026"
      >
        <section>
          <h2>1. Scope</h2>
          <p>
            This policy covers the algorithco guard command-line software and
            this website. The product remains local-first. In account-mode
            builds, cloud and team features use the central Algorithco account
            service; payment details are handled there and must never be entered
            into Guard.
          </p>
        </section>
        <section>
          <h2>2. Processing modes</h2>
          <div className="table-scroll">
            <table className="cli-table">
              <thead>
                <tr>
                  <th>Mode</th>
                  <th>What happens</th>
                  <th>Network</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>
                    <code>local-only</code> (default)
                  </td>
                  <td>Policy checks and audit records stay on your device.</td>
                  <td>None for decision processing.</td>
                </tr>
                <tr>
                  <td>
                    <code>redacted</code> (opt-in)
                  </td>
                  <td>
                    Sensitive values are masked before an enabled provider
                    receives context.
                  </td>
                  <td>Only after configuration and consent.</td>
                </tr>
                <tr>
                  <td>
                    <code>full</code> (opt-in)
                  </td>
                  <td>
                    Unredacted context may be sent after an additional explicit
                    confirmation.
                  </td>
                  <td>Only to the provider you configure.</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>
        <section>
          <h2>3. Data stored locally</h2>
          <p>
            The software may store configuration, policy rules, decision
            metadata, reasons, latency, and redacted audit events under the
            algorithco guard home directory. Use <code>algo log</code> to review
            records and <code>algo uninstall</code> to remove installed hooks.
            Removing local data remains under your operating-system control.
          </p>
        </section>
        <section>
          <h2>4. Website and optional services</h2>
          <p>
            Static pages can be viewed without an account. Choosing account
            sign-in sends an OIDC authorization request to the configured
            Algorithco account service. Guard keeps browser tokens in memory
            only and sends the access token to the Guard backend for verified
            cloud/team requests. Guard reads effective entitlement data but does
            not store prices, subscriptions, payment details, or card data.
            Hosted-service legal entity, processors, locations, retention
            periods, and contact details remain publication TODOs before general
            availability.
          </p>
          <p>
            For entitlement lookup, the backend sends the account service the
            opaque account subject in the documented request path and a
            read-only Guard service credential. Command text, policy input, and
            local audit contents are not part of that request. The backend keeps
            bounded JWKS and entitlement caches in memory; it does not register
            a billing webhook or persist billing events.
          </p>
        </section>
        <section>
          <h2>5. Your controls</h2>
          <ul>
            <li>Keep the default local-only mode.</li>
            <li>
              Inspect intended egress with <code>algo log --show-egress</code>.
            </li>
            <li>
              Pause enforcement or uninstall without relying on the daemon.
            </li>
            <li>
              Do not enable an external provider until you accept its terms.
            </li>
          </ul>
        </section>
        <section>
          <h2>6. Security, children, and changes</h2>
          <p>
            We use layered technical and release controls described on the{" "}
            <Link to="/security">security page</Link>. The product is intended
            for software-development use and not directed to children. Material
            changes will be dated here. A verified privacy contact and legal
            entity must be published before hosted general availability.
          </p>
        </section>
      </LegalLayout>
    </>
  );
}
