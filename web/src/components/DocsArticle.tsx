import type { ReactNode } from "react";
import { routeMeta } from "../lib/routes";
import { MotionReveal } from "./MotionReveal";
import { PrevNext } from "./PrevNext";

export function DocsArticle({
  eyebrow,
  title,
  summary,
  path,
  children,
}: {
  eyebrow: string;
  title: string;
  summary: string;
  path: string;
  children: ReactNode;
}): JSX.Element {
  const meta = routeMeta(path);
  return (
    <article className="docs-article">
      <MotionReveal className="docs-hero">
        <p className="kicker">{eyebrow}</p>
        <h1>{title}</h1>
        <p className="lede">{summary}</p>
      </MotionReveal>
      <div className="docs-prose">{children}</div>
      <PrevNext prev={meta?.prev} next={meta?.next} />
    </article>
  );
}
