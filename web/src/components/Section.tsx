import type * as React from "react";

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
    <section id={id} className="section reveal">
      <div className="wrap">
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
        <div className="section-body">{children}</div>
      </div>
    </section>
  );
}
