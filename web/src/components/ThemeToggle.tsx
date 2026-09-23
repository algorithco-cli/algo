import Moon from "lucide-react/icons/moon.mjs";
import Sun from "lucide-react/icons/sun.mjs";
import * as React from "react";

export function useTheme(): ["light" | "dark", () => void] {
  const [theme, setTheme] = React.useState<"light" | "dark">(() => {
    if (typeof document !== "undefined") {
      const a = document.documentElement.getAttribute("data-theme");
      if (a === "dark" || a === "light") return a;
    }
    return "dark";
  });
  const toggle = React.useCallback(() => {
    setTheme((t) => {
      const next = t === "light" ? "dark" : "light";
      document.documentElement.setAttribute("data-theme", next);
      try {
        window.localStorage.setItem("ag-theme", next);
      } catch {
        /* storage unavailable — theme still applies for this session */
      }
      return next;
    });
  }, []);
  return [theme, toggle];
}

export function ThemeToggle(): JSX.Element {
  const [theme, toggle] = useTheme();
  return (
    <button
      type="button"
      className="theme-toggle"
      onClick={toggle}
      aria-label={
        theme === "light" ? "Switch to dark mode" : "Switch to light mode"
      }
    >
      {theme === "light" ? (
        <Moon size={17} aria-hidden />
      ) : (
        <Sun size={17} aria-hidden />
      )}
    </button>
  );
}
