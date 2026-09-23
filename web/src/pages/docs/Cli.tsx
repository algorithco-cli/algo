import { Breadcrumbs } from "../../components/Breadcrumbs";
import { PrevNext } from "../../components/PrevNext";
import { Section } from "../../components/Section";
import { Seo } from "../../components/Seo";
import { CLI_ROWS } from "../../data/content";
import { useRevealOnMount } from "../../hooks/useRevealOnMount";

export default function Cli(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/docs/cli" />
      <Breadcrumbs
        trail={[
          { label: "Home", to: "/" },
          { label: "Docs", to: "/docs" },
          { label: "CLI reference" },
        ]}
      />
      <Section
        id="cli"
        kicker="Docs"
        title="CLI reference"
        lede="Nine commands. Additive install, byte-identical uninstall."
      >
        <div className="card" id="cli-table">
          <h3>Commands</h3>
          <table className="cli-table">
            <thead>
              <tr>
                <th>Command</th>
                <th>Purpose</th>
              </tr>
            </thead>
            <tbody>
              {CLI_ROWS.map(([cmd, desc]) => (
                <tr key={cmd} id={`algo-${cmd.split(" ")[1]}`}>
                  <td>
                    <code>{cmd}</code>
                  </td>
                  <td className="muted">{desc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="card" id="paths">
          <h3>Local paths</h3>
          <pre>
            <code>{`~/.algo/            # home
~/.algo/algo.sock  # daemon socket (0600)
~/.algo/audit.db   # SQLite audit log (WAL)`}</code>
          </pre>
          <p className="muted small">
            Additive and reversible: agent configs are backed up (
            <code>*.algo-backup-&lt;ts&gt;</code>) and{" "}
            <code>algo uninstall</code> restores them byte-identical.
          </p>
        </div>
        <PrevNext prev="/docs/install" />
      </Section>
    </>
  );
}
