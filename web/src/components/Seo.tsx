import * as React from "react";
import { routeMeta } from "../lib/routes";
import { OG_IMAGE, SITE_URL, canonicalFor } from "../lib/site";

export function Seo({ path }: { path: string }): JSX.Element {
  const meta = routeMeta(path);
  const title = meta?.title ?? "Algorithco Guard — safer AI coding agents";
  const description =
    meta?.description ??
    "Control layer for AI coding agents. Hard-deny destructive commands, fail-safe ask, redact-first privacy.";
  const canonical = canonicalFor(path);
  const noindex = meta?.indexable === false;
  React.useEffect(() => {
    document.title = title;
    const set = (sel: string, attr: string, val: string): void => {
      let el = document.head.querySelector(sel) as
        | HTMLMetaElement
        | HTMLLinkElement
        | null;
      if (!el) {
        if (sel.startsWith("meta")) {
          el = document.createElement("meta") as HTMLMetaElement;
          if (sel.includes('name="description"'))
            (el as HTMLMetaElement).name = "description";
          if (sel.includes('property="og:')) {
            const m = sel.match(/property="([^"]+)"/);
            if (m) (el as HTMLMetaElement).setAttribute("property", m[1]);
          }
          document.head.appendChild(el);
        } else if (sel.startsWith("link")) {
          el = document.createElement("link") as HTMLLinkElement;
          (el as HTMLLinkElement).rel = "canonical";
          document.head.appendChild(el);
        } else return;
      }
      el.setAttribute(attr, val);
    };
    set('meta[name="description"]', "content", description);
    set('link[rel="canonical"]', "href", canonical);
    let robots = document.head.querySelector(
      'meta[name="robots"]',
    ) as HTMLMetaElement | null;
    if (noindex) {
      if (!robots) {
        robots = document.createElement("meta");
        robots.name = "robots";
        document.head.appendChild(robots);
      }
      robots.content = "noindex,follow";
    } else if (robots) {
      robots.remove();
    }
  }, [title, description, canonical, noindex]);
  return (
    <>
      <link rel="canonical" href={canonical} />
      <meta property="og:url" content={canonical} />
      <meta property="og:title" content={title} />
      <meta property="og:description" content={description} />
      <meta property="og:image" content={OG_IMAGE} />
      <meta property="og:site_name" content="Algorithco Guard" />
      <meta name="twitter:card" content="summary_large_image" />
      <meta name="twitter:title" content={title} />
      <meta name="twitter:description" content={description} />
      <meta name="twitter:image" content={OG_IMAGE} />
      <meta name="site-url" content={SITE_URL} />
    </>
  );
}
