import { Breadcrumbs } from "../../components/Breadcrumbs";
import { InstallTabs } from "../../components/InstallTabs";
import { PrevNext } from "../../components/PrevNext";
import { Section } from "../../components/Section";
import { Seo } from "../../components/Seo";
import { useRevealOnMount } from "../../hooks/useRevealOnMount";

export default function Install(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/docs/install" />
      <Breadcrumbs
        trail={[
          { label: "Home", to: "/" },
          { label: "Docs", to: "/docs" },
          { label: "Install" },
        ]}
      />
      <Section
        id="install"
        kicker="Docs"
        title="Install in ~30s"
        lede="Diff + per-agent consent before anything changes. Backups restore byte-identical."
      >
        <InstallTabs />
        <details className="verify-details">
          <summary>Prefer to read before you run? Verify first.</summary>
          <pre>
            <code>{`curl -fsSL http://127.0.0.1:3007/install.sh -o /tmp/algo-install.sh
less /tmp/algo-install.sh   # plain script — read it before you trust it
sh /tmp/algo-install.sh`}</code>
          </pre>
          <p className="muted small">
            Piping <code>curl | sh</code> asks for trust, so don&apos;t start
            there. And <code>algo init</code> itself shows a diff and asks
            per-agent consent before changing anything —{" "}
            <code>algo uninstall</code> restores byte-identical.
          </p>
        </details>
        <PrevNext prev="/docs" next="/docs/cli" />
      </Section>
    </>
  );
}
