import * as React from "react";
import { useLocation } from "react-router-dom";

const HASH_TO_PATH: Record<string, string> = {
  "#features": "/features",
  "#how": "/how",
  "#pricing": "/pricing",
  "#roadmap": "/roadmap",
  "#login": "/login",
  "#docs": "/docs",
  "#install": "/docs/install",
  "#cli": "/docs/cli",
  "#paths": "/docs/cli",
  "#privacy": "/privacy",
  "#faq": "/faq",
  "#demo": "/#demo",
  "#top": "/",
};

export function ScrollToTop(): null {
  const { pathname, hash } = useLocation();
  React.useEffect(() => {
    // One-time legacy hash redirect (#features → /features).
    if (hash && HASH_TO_PATH[hash] && pathname === "/") {
      const target = HASH_TO_PATH[hash];
      if (target.startsWith("/#")) {
        document.getElementById("demo")?.scrollIntoView();
        return;
      }
      window.location.replace(target);
      return;
    }
    const reduce = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (hash) {
      document
        .getElementById(hash.slice(1))
        ?.scrollIntoView({ behavior: reduce ? "auto" : "smooth" });
      return;
    }
    window.scrollTo({ top: 0, behavior: "auto" });
  }, [pathname, hash]);
  return null;
}
