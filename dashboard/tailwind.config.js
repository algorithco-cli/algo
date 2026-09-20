/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class", '[data-theme="dark"]'],
  content: ["./index.html", "./src/**/*.{ts,tsx,js,jsx}"],
  theme: {
    extend: {
      colors: {
        // Direct token colors (Variant 1 single source)
        brand: "var(--ag-brand)",
        bg: "var(--ag-bg)",
        surface: "var(--ag-surface)",
        border: "var(--ag-border)",
        text: "var(--ag-text)",
        muted: "var(--ag-text-muted)",
        allow: "var(--ag-allow)",
        ask: "var(--ag-ask)",
        deny: "var(--ag-deny)",
        // shadcn/ui aliases → brand / surface / bg / border / text per design-tokens.md §3
        primary: {
          DEFAULT: "var(--ag-brand)",
          foreground: "#FFFFFF",
        },
        background: "var(--ag-bg)",
        card: {
          DEFAULT: "var(--ag-surface)",
          foreground: "var(--ag-text)",
        },
        foreground: "var(--ag-text)",
        "muted-foreground": "var(--ag-text-muted)",
        destructive: "var(--ag-deny)",
        success: "var(--ag-allow)",
        warning: "var(--ag-ask)",
      },
      borderColor: {
        DEFAULT: "var(--ag-border)",
      },
      fontFamily: {
        sans: ["Inter", "system-ui", "-apple-system", "sans-serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
      },
    },
  },
  plugins: [],
};
