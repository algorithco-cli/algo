import type { ReactNode } from "react";
import { Breadcrumbs } from "./Breadcrumbs";
import { MotionReveal } from "./MotionReveal";

interface LegalLayoutProps {
  eyebrow: string;
  title: string;
  summary: string;
  reviewed: string;
  children: ReactNode;
}

export function LegalLayout({
  eyebrow,
  title,
  summary,
  reviewed,
  children,
}: LegalLayoutProps): JSX.Element {
  return (
    <div className="wrap legal-shell">
      <Breadcrumbs trail={[{ label: "Home", to: "/" }, { label: title }]} />
      <MotionReveal className="legal-hero">
        <p className="kicker">{eyebrow}</p>
        <h1>{title}</h1>
        <p className="legal-summary">{summary}</p>
        <p className="muted small">Last reviewed {reviewed}</p>
      </MotionReveal>
      <div className="legal-grid">
        <aside className="legal-note">
          <strong>Plain-language commitment</strong>
          <p>
            Local means local. Optional network features must be visible,
            deliberate, and inspectable.
          </p>
        </aside>
        <article className="legal-article">{children}</article>
      </div>
    </div>
  );
}
