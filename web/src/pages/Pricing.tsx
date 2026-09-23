import { Link } from "react-router-dom";
import ElectricBorder from "../components/ElectricBorder";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

export default function Pricing(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/pricing" />
      <Section
        id="pricing"
        kicker="Pricing"
        title="Start free."
        accent="Scale when you trust it."
        lede="Local is free forever. Team and Enterprise arrive with Phase 3 cloud — pricing below is the target, not checkout."
      >
        <div className="grid3">
          <ElectricBorder
            color="#6D4AFF"
            speed={1.15}
            chaos={0.28}
            borderRadius={16}
            style={{ borderRadius: 16 }}
          >
            <div
              className="card price featured"
              id="local"
              style={{ border: 0, boxShadow: "none" }}
            >
              <h3>Local — $0 forever</h3>
              <p className="badge badge-allow">Available now</p>
              <ul className="tight-list">
                <li>Hard-deny + shadow mode</li>
                <li>Ask-on-error, local-only</li>
                <li>algo why / status / log</li>
                <li>One-step pause + uninstall</li>
              </ul>
              <Link to="/docs/install" className="btn btn-primary">
                Install free
              </Link>
            </div>
          </ElectricBorder>
          <ElectricBorder
            color="#8E77FF"
            speed={1.1}
            chaos={0.26}
            borderRadius={16}
            style={{ borderRadius: 16 }}
          >
            <div
              className="card price planned"
              id="team"
              style={{ border: 0, boxShadow: "none" }}
            >
              <h3>Team — $19/seat/mo</h3>
              <p className="badge badge-ask">Planned P3</p>
              <ul className="tight-list">
                <li>Dashboard + SSE live tail</li>
                <li>Signed bundles + dry-run</li>
                <li>Per-user stats, OAuth + org</li>
              </ul>
              <Link to="/login" className="btn btn-ghost">
                Join waitlist
              </Link>
            </div>
          </ElectricBorder>
          <ElectricBorder
            color="#A68FFF"
            speed={1.05}
            chaos={0.24}
            borderRadius={16}
            style={{ borderRadius: 16 }}
          >
            <div
              className="card price planned"
              id="enterprise"
              style={{ border: 0, boxShadow: "none" }}
            >
              <h3>Enterprise — Custom</h3>
              <p className="badge badge-ask">Planned P3</p>
              <ul className="tight-list">
                <li>SSO / SCIM, audit export</li>
                <li>ZDR, DPA, SBOM + provenance</li>
              </ul>
              <Link to="/faq" className="btn btn-ghost">
                Talk to us
              </Link>
            </div>
          </ElectricBorder>
        </div>
        <p className="muted small">
          Team/Enterprise billing via external MoR (ADR-0006). See{" "}
          <Link to="/roadmap">roadmap P3</Link> and{" "}
          <Link to="/faq">FAQ cost</Link>.
        </p>
      </Section>
    </>
  );
}
