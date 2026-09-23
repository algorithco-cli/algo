import * as React from "react";

export const Counter = React.memo(function Counter({
  to,
  prefix,
  suffix,
}: {
  to: number;
  prefix?: string;
  suffix?: string;
}): JSX.Element {
  const [v, setV] = React.useState(0);
  const ref = React.useRef<HTMLSpanElement>(null);
  React.useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const reduce = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    const saveData = (
      navigator as Navigator & { connection?: { saveData?: boolean } }
    ).connection?.saveData;
    const smallScreen = window.matchMedia?.("(max-width: 640px)").matches;
    if (reduce || saveData || smallScreen) {
      setV(to);
      return;
    }
    let raf = 0;
    const io = new IntersectionObserver(
      ([entry]) => {
        if (!entry.isIntersecting) return;
        io.disconnect();
        const t0 = performance.now();
        const dur = 1200;
        const tick = (t: number): void => {
          const p = Math.min(1, (t - t0) / dur);
          setV(Math.round(to * (1 - (1 - p) ** 3)));
          if (p < 1) raf = requestAnimationFrame(tick);
        };
        raf = requestAnimationFrame(tick);
      },
      { threshold: 0.4 },
    );
    io.observe(el);
    return () => {
      io.disconnect();
      cancelAnimationFrame(raf);
    };
  }, [to]);
  return (
    <span ref={ref}>
      {prefix ?? ""}
      {v}
      {suffix ?? ""}
    </span>
  );
});
