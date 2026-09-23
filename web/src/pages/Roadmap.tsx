import Check from "lucide-react/icons/check.mjs";
import Cloud from "lucide-react/icons/cloud.mjs";
import LayoutGrid from "lucide-react/icons/layout-grid.mjs";
import Rocket from "lucide-react/icons/rocket.mjs";
import { Link } from "react-router-dom";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { ROADMAP } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

const ICONS: Record<string, typeof Check> = {
  check: Check,
  rocket: Rocket,
  cloud: Cloud,
  grid: LayoutGrid,
};

export default function Roadmap(): JSX.Element {
  useRevealOnMount();
  return (
    <>
      <Seo path="/roadmap" />
      <Section
        id="roadmap"
        kicker="Roadmap"
        title="Where guard is"
        accent="going"
        lede="Strict build order proto → core → agent → cloud. Each phase ships only when gates pass."
      >
        <div className="timeline">
          {ROADMAP.map((r) => {
            const Icon = ICONS[r.icon] ?? Check;
            return (
              <div key={r.phase} className={`tl-item tl-${r.state}`}>
                <span className="tl-dot" aria-hidden>
                  <Icon size={16} />
                </span>
                <div>
                  <p className="muted small">{r.phase}</p>
                  <h3>{r.title}</h3>
                  <p className="muted">{r.body}</p>
                </div>
              </div>
            );
          })}
        </div>
        <p className="muted small">
          Start on P1 <Link to="/docs/install">today</Link> · pricing for P3{" "}
          <Link to="/pricing">here</Link>.
        </p>
      </Section>
    </>
  );
}
