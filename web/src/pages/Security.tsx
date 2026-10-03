import { Link } from "react-router-dom";
import { LegalLayout } from "../components/LegalLayout";
import { Seo } from "../components/Seo";

export default function Security(): JSX.Element {
  return (
    <>
      <Seo path="/security" />
      <LegalLayout
        eyebrow="Trust center"
        title="Security"
        summary="The product controls, release checks, and reporting path that protect a local command guard."
        reviewed="October 3, 2026"
      >
        <section>
          <h2>Fail-safe decisions</h2>
          <p>
            New I/O, parsing, and timeout failures must resolve to{" "}
            <code>ask</code>, never silently to <code>allow</code>. Hard-deny
            rules cannot be overridden by convenience rules.
          </p>
        </section>
        <section>
          <h2>Local-first boundary</h2>
          <p>
            Decision processing defaults to the device. Optional egress is
            mode-gated, redacted before transport where configured, and visible
            through the audit tooling described in the{" "}
            <Link to="/docs/privacy-security">privacy and security guide</Link>.
          </p>
        </section>
        <section>
          <h2>Account boundary</h2>
          <p>
            Cloud and team features use the central Algorithco account service
            for identity and entitlements. The browser keeps tokens in memory;
            the CLI keeps them in the operating-system credential vault; and the
            Guard backend verifies access tokens against discovered ES256 keys.
            Guard does not store prices, subscriptions, payment details, or
            billing webhook events.
          </p>
          <p>
            Authentication fails closed. Entitlement outages fall back to a
            still-valid last-known entitlement or the free tier, and cannot
            change an <code>allow</code>, <code>ask</code>, or <code>deny</code>{" "}
            policy decision.
          </p>
        </section>
        <section>
          <h2>Release controls</h2>
          <ul>
            <li>
              Formatting, linting, unit, integration, property, and fuzz checks.
            </li>
            <li>
              Dependency audit, secret scanning, signed artifacts, provenance,
              and SBOMs.
            </li>
            <li>
              Contract compatibility checks and human review for sensitive
              policy changes.
            </li>
          </ul>
        </section>
        <section>
          <h2>Report a vulnerability</h2>
          <p>
            Follow the private reporting instructions in the repository’s{" "}
            <code>SECURITY.md</code>. Do not disclose an unpatched issue in a
            public ticket. A verified security email and response SLA must be
            published before general availability.
          </p>
        </section>
      </LegalLayout>
    </>
  );
}
