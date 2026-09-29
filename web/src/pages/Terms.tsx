import { Link } from "react-router-dom";
import { LegalLayout } from "../components/LegalLayout";
import { Seo } from "../components/Seo";

export default function Terms(): JSX.Element {
  return (
    <>
      <Seo path="/terms" />
      <LegalLayout
        eyebrow="Legal"
        title="Terms of use"
        summary="The rules for using the algorithco guard website and local software during the private preview."
        reviewed="September 28, 2026"
      >
        <section>
          <h2>1. Preview status</h2>
          <p>
            The software and website are under active development. Hosted
            accounts, billing, organization management, and cloud features are
            previews unless a separate signed agreement says otherwise.
          </p>
        </section>
        <section>
          <h2>2. License and acceptable use</h2>
          <p>
            Your right to use source code and binaries is governed by the
            license distributed with the applicable package. You may not use the
            service to break the law, compromise systems without authorization,
            evade safeguards, or interfere with other users.
          </p>
        </section>
        <section>
          <h2>3. Your responsibility</h2>
          <p>
            Guard provides decision support, not a guarantee that every command
            is safe or harmful. Review prompts and outputs, protect credentials,
            test policies, and maintain backups. An <em>allow</em> decision is
            not professional, legal, security, or compliance advice.
          </p>
        </section>
        <section>
          <h2>4. Availability and changes</h2>
          <p>
            Preview features may change, pause, or be withdrawn. We aim to
            preserve documented local contracts and provide migration notes, but
            do not promise uninterrupted or error-free operation.
          </p>
        </section>
        <section>
          <h2>5. Disclaimers and liability</h2>
          <p>
            To the maximum extent permitted by applicable law, preview software
            is provided “as is” without warranties. Liability terms, governing
            law, the contracting legal entity, and a formal notice address must
            be completed by counsel before any paid hosted service launches.
          </p>
        </section>
        <section>
          <h2>6. Privacy and security</h2>
          <p>
            Read the <Link to="/privacy">privacy policy</Link> and{" "}
            <Link to="/security">security page</Link> before enabling optional
            network processing. These terms do not override your rights under
            applicable law.
          </p>
        </section>
      </LegalLayout>
    </>
  );
}
