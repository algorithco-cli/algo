import type * as React from "react";
import { MotionReveal } from "./MotionReveal";

export function Section({
  id,
  kicker,
  title,
  accent,
  lede,
  children,
}: {
  id: string;
  kicker: string;
  title: string;
  accent?: string;
  lede?: string;
  children: React.ReactNode;
}): JSX.Element {
  return (
    <section id={id} className="section">
      <div className="wrap">
        <MotionReveal>
          <p className="kicker">{kicker}</p>
          <h2 className="section-title">
            {title}
            {accent ? (
              <>
                {" "}
                <span className="accent">{accent}</span>
              </>
            ) : null}
          </h2>
          {lede ? <p className="muted section-lede">{lede}</p> : null}
        </MotionReveal>
        <MotionReveal className="section-body" delay={0.08}>
          {children}
        </MotionReveal>
      </div>
    </section>
  );
}
