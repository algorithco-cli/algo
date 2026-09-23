import * as React from "react";

export function useScrolled(threshold = 8): boolean {
  const [scrolled, setScrolled] = React.useState(false);
  React.useEffect(() => {
    let raf = 0;
    let last = window.scrollY > threshold;
    setScrolled(last);
    const onScroll = (): void => {
      cancelAnimationFrame(raf);
      raf = requestAnimationFrame(() => {
        const next = window.scrollY > threshold;
        if (next !== last) {
          last = next;
          setScrolled(next);
        }
      });
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => {
      window.removeEventListener("scroll", onScroll);
      cancelAnimationFrame(raf);
    };
  }, [threshold]);
  return scrolled;
}
