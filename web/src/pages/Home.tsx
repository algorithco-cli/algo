import ArrowRight from "lucide-react/icons/arrow-right.mjs";
import Check from "lucide-react/icons/check.mjs";
import X from "lucide-react/icons/x.mjs";
import { Link } from "react-router-dom";
import BlurText from "../components/BlurText";
import { Counter } from "../components/Counter";
import Grainient from "../components/Grainient";
import { PipelineDiagram } from "../components/PipelineDiagram";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { STATS } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

const STAT_ICON = { zap: 0, shield: 1, flask: 2, check: 3 } as const;

export default function Home(): JSX.Element {
  useRevealOnMount();
  // Grainient tuned to Variant 1 — screenshot settings + our brand palette:
  // screenshot: ts 3.75 / cb 0.11 / wf 2.3 / ws 0.8 / wa 22 / blend 0/0.05 / rot 500 / noise 1 / grain 0.1/2 off / contrast 2.5 / gamma 0.7 / sat 0.75 / zoom 1.15
  // brand-adapted: our --ag-brand #6D4AFF (light) / #8E77FF (dark) + mid #7258df-purple so dark hero tints purple, not pink.
  return (
    <>
      <Seo path="/" />
      <section className="hero" id="top">
        <div className="hero-grainient" aria-hidden>
          <Grainient
            className="hero-grainient"
            color1="#d588d3"
            color2="#7258df"
            color3="#ad74e1"
            timeSpeed={3.75}
            colorBalance={0.11}
            warpStrength={1}
            warpFrequency={2.3}
            warpSpeed={0.8}
            warpAmplitude={22}
            blendAngle={0}
            blendSoftness={0.05}
            rotationAmount={500}
            noiseScale={1}
            grainAmount={0.1}
            grainScale={2}
            grainAnimated={false}
            contrast={2.5}
            gamma={0.7}
            saturation={0.75}
            centerX={0}
            centerY={0}
            zoom={1.15}
          />
          <div className="hero-grainient-fade" aria-hidden />
        </div>
        <div className="wrap hero-grid hero-grid--single">
          <div className="hero-copy hero-copy--centered">
            <BlurText
              text="Ship agents you don't have to babysit."
              delay={120}
              animateBy="words"
              direction="top"
              threshold={0.1}
              stepDuration={0.35}
              className="hero-title"
            />
            <p className="muted hero-sub">
              Guard judges every shell command — hard-deny, ask, or allow — in
              under 3ms, and logs the reason.
            </p>
            <div className="hero-actions">
              <Link to="/docs/install" className="btn btn-primary btn-lg">
                Install free <ArrowRight size={16} aria-hidden />
              </Link>
              <Link to="/docs" className="btn btn-ghost btn-lg">
                View docs
              </Link>
            </div>
            <div className="hero-meta muted">
              <span>~30s install</span> · <span>&lt;3ms local decisions</span> ·{" "}
              <span>local-only by default</span> · <span>174+ tests green</span>
            </div>
          </div>
        </div>
      </section>

      <Section
        id="problem"
        kicker="Why guard"
        title="Agents act fast."
        accent="Mistakes compound faster."
        lede="One piped curl, one recursive chmod, one leaked key — and the agent keeps going. Guard puts a millisecond checkpoint in front of every shell call."
      >
        <div className="grid2">
          <div className="card problem">
            <h3>Without guard</h3>
            <ul className="tight-list">
              <li>
                <X size={14} aria-hidden /> <code>curl … | sh</code> runs before
                you see it
              </li>
              <li>
                <X size={14} aria-hidden /> <code>rm -rf /</code> is one typo
                away
              </li>
              <li>
                <X size={14} aria-hidden /> secrets leave in prompts, no record
              </li>
            </ul>
          </div>
          <div className="card solution">
            <h3>With guard</h3>
            <ul className="tight-list">
              <li>
                <Check size={14} aria-hidden /> 13 versioned hard-deny rules, no
                overrides
              </li>
              <li>
                <Check size={14} aria-hidden /> ask-on-error, shadow
                would-have-N digest
              </li>
              <li>
                <Check size={14} aria-hidden /> on-machine redact +{" "}
                <code>~/.algo/audit.db</code> log
              </li>
            </ul>
          </div>
        </div>
      </Section>

      <Section
        id="features"
        kicker="Features"
        title="Safety that stays"
        accent="out of the way"
        lede="Four properties, each pinned by tests and CI gates. Full detail on the features page."
      >
        <div className="bento">
          <div className="card bento-tile">
            <h3>Safer</h3>
            <p className="muted">
              Hard-deny before it runs. Rules outrank models; no profile
              overrides a deny.
            </p>
            <Link to="/features">Explore features →</Link>
          </div>
          <div className="card bento-tile">
            <h3>Quieter</h3>
            <p className="muted">
              Shadow-first + would-have-blocked digest. Quiet unless it needs
              you.
            </p>
            <Link to="/features">Explore features →</Link>
          </div>
          <div className="card bento-tile">
            <h3>Auditable</h3>
            <p className="muted">
              algo why = action + reason + confidence + source + latency.
            </p>
            <Link to="/how">How it works →</Link>
          </div>
          <div className="card bento-tile">
            <h3>Integrations</h3>
            <p className="muted">
              Hooks, plugins, MCP. Claude Code now; Codex and OpenCode planned.
            </p>
            <Link to="/docs/install">Install →</Link>
          </div>
        </div>
        <div className="stats-band">
          {STATS.map((s) => (
            <div key={s.label} className="stat">
              <span className="stat-value">
                <Counter to={s.value} prefix={s.prefix} suffix={s.suffix} />
              </span>
              <span className="muted small">{s.label}</span>
              <span hidden>{STAT_ICON[s.icon as keyof typeof STAT_ICON]}</span>
            </div>
          ))}
        </div>
      </Section>

      <Section
        id="how"
        kicker="How it works"
        title="One pipeline,"
        accent="milliseconds"
        lede="Hook, parse, decide, render — every hop attaches source + latency."
      >
        <div className="card pipeline-card">
          <PipelineDiagram />
        </div>
        <p>
          <Link to="/how">How it works in detail →</Link>
        </p>
      </Section>
    </>
  );
}
