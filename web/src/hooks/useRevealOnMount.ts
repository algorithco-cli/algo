import * as React from "react";

export function useRevealOnMount(): void {
  React.useEffect(() => {
    const els = Array.from(document.querySelectorAll(".reveal:not(.visible)"));
    if (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      for (const e of els) e.classList.add("visible");
      return;
    }
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (e.isIntersecting) {
            e.target.classList.add("visible");
            io.unobserve(e.target);
          }
        }
      },
      { threshold: 0.1, rootMargin: "0px 0px -10% 0px" },
    );
    for (const e of els) io.observe(e);
    return () => io.disconnect();
  });
}
