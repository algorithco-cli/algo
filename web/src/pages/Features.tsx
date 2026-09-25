import {
  Moon02Icon,
  PuzzleIcon,
  Search01Icon,
  Shield01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useCallback, useState } from "react";
import type { ReactNode } from "react";
import { Counter } from "../components/Counter";
import { FeatureModal } from "../components/FeatureModal";
import type { FeatureModalLink } from "../components/FeatureModal";
import { FolderFloat } from "../components/FolderFloat";
import { Section } from "../components/Section";
import { Seo } from "../components/Seo";
import { VerdictDemo } from "../components/VerdictDemo";
import { STATS } from "../data/content";
import { useRevealOnMount } from "../hooks/useRevealOnMount";

const FEATURE_ITEMS = [
  { label: "Safer", value: "safer" },
  { label: "Quieter", value: "quieter" },
  { label: "Auditable", value: "auditable" },
  { label: "Integrations", value: "integrations" },
];

const FEATURE_ORDER = ["safer", "quieter", "auditable", "integrations"];

const FEATURE_INFO: Record<
  string,
  { title: string; body: ReactNode; icon: ReactNode; link?: FeatureModalLink }
> = {
  safer: {
    title: "Safer — 13 hard-deny rules",
    body: (
      <p>
        <code>rm -rf /</code>, <code>mkfs</code>, <code>dd</code> to devices,
        fork bombs, <code>curl|sh</code> and more are denied before they run.
        Rules outrank models — no profile overrides a deny. Try it live below.
      </p>
    ),
    icon: <HugeiconsIcon icon={Shield01Icon} size={22} strokeWidth={1.8} />,
  },
  quieter: {
    title: "Quieter — shadow-first",
    body: (
      <p>
        Ships in shadow mode: records would-have-blocked without blocking
        anything. Flip to enforce only with an explicit{" "}
        <code>algo enforce on</code>. Quiet unless it needs you.
      </p>
    ),
    icon: <HugeiconsIcon icon={Moon02Icon} size={22} strokeWidth={1.8} />,
  },
  auditable: {
    title: "Auditable — algo why",
    body: (
      <p>
        Every decision logs action + reason + confidence + source + latency to{" "}
        <code>~/.algo/audit.db</code>. Inspect egress any time with{" "}
        <code>algo log --show-egress</code>.
      </p>
    ),
    icon: <HugeiconsIcon icon={Search01Icon} size={22} strokeWidth={1.8} />,
    link: { to: "/how", label: "How the pipeline works" },
  },
  integrations: {
    title: "Integrations — hooks, plugins, MCP",
    body: (
      <p>
        Claude Code Bash hooks today. Codex and OpenCode adapters arrive in
        Phase 4 on the versioned proto contract, so adapters stay thin.
      </p>
    ),
    icon: <HugeiconsIcon icon={PuzzleIcon} size={22} strokeWidth={1.8} />,
    link: { to: "/docs/install", label: "Install for Claude Code" },
  },
};

export default function Features(): JSX.Element {
  useRevealOnMount();
  const [active, setActive] = useState<number>(-1);
  const handleSelect = useCallback((value: string): void => {
    const i = FEATURE_ORDER.indexOf(value);
    if (i >= 0) setActive(i);
  }, []);
  const handleClose = useCallback(() => setActive(-1), []);
  const handleStep = useCallback((i: number): void => {
    if (i >= 0 && i < FEATURE_ORDER.length) setActive(i);
  }, []);
  const key = active < 0 ? null : FEATURE_ORDER[active];
  const info = key === null ? null : FEATURE_INFO[key];
  return (
    <>
      <Seo path="/features" />
      <Section
        id="features"
        kicker="Features"
        title="Safety that stays"
        accent="out of the way"
        lede="Four properties, each pinned by tests and CI gates — not claims."
      >
        <div className="features-folder">
          <FolderFloat
            items={FEATURE_ITEMS}
            label="Guard pillars"
            trigger="click"
            width={280}
            height={208}
            radius={16}
            spread={250}
            lift={32}
            onSelect={handleSelect}
          />
          <p className="muted small features-folder-hint">
            Click the folder — then click a pill for details. You can drag them
            around.
          </p>
        </div>
        {info && key ? (
          <FeatureModal
            kicker="Guard pillar"
            icon={info.icon}
            title={info.title}
            body={info.body}
            link={info.link}
            steps={FEATURE_ORDER}
            index={active}
            onStep={handleStep}
            onClose={handleClose}
          />
        ) : null}
        <VerdictDemo />
        <div className="stats-band">
          {STATS.map((s) => (
            <div key={s.label} className="stat">
              <span className="stat-value">
                <Counter to={s.value} prefix={s.prefix} suffix={s.suffix} />
              </span>
              <span className="muted small">{s.label}</span>
            </div>
          ))}
        </div>
      </Section>
    </>
  );
}
