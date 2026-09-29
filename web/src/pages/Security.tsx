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
        reviewed="September 28, 2026"
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
