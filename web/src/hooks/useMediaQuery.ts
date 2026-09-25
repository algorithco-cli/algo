import * as React from "react";

/**
 * SSR-safe matchMedia hook. Defaults to `false` on the server (and before
 * mount), then subscribes with a change listener so it tracks window resize
 * and orientation change.
 */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = React.useState(false);

  React.useEffect(() => {
    const mql = window.matchMedia(query);
    const onChange = (): void => setMatches(mql.matches);
    onChange();
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [query]);

  return matches;
}
