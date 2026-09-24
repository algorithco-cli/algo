import {
  QueryClient,
  QueryClientProvider,
  useQuery,
} from "@tanstack/react-query";
import {
  Link,
  Outlet,
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import * as React from "react";
import { HistoryTable } from "./components/HistoryTable";
import { LiveFeed } from "./components/LiveFeed";
import { PolicyEditor } from "./components/PolicyEditor";
import { StatsView } from "./components/StatsView";
import { fetchAuditHistoryWithFallback } from "./lib/fallback";

// ─── Theme toggle (light/dark) using data-theme + Variant 1 tokens ───────────

function ThemeToggle(): JSX.Element {
  const [theme, setTheme] = React.useState<"light" | "dark">(() => {
    if (typeof document !== "undefined") {
      const attr = document.documentElement.getAttribute("data-theme");
      if (attr === "dark" || attr === "light") return attr;
      if (window.matchMedia?.("(prefers-color-scheme: dark)").matches)
        return "dark";
    }
    return "light";
  });

  React.useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return (
    <button
      type="button"
      onClick={() => setTheme((t) => (t === "light" ? "dark" : "light"))}
      className="inline-flex h-8 items-center gap-1.5 rounded-md border px-2.5 text-xs"
      style={{
        borderColor: "var(--ag-border)",
        background: "var(--ag-surface)",
        color: "var(--ag-text)",
      }}
      aria-label={`Switch to ${theme === "light" ? "dark" : "light"} theme`}
      title="Toggle light / dark (Variant 1 tokens)"
    >
      <span aria-hidden>{theme === "light" ? "☾" : "☀"}</span>
      <span className="hidden sm:inline">
        {theme === "light" ? "Dark" : "Light"}
      </span>
    </button>
  );
}

// ─── Layout ──────────────────────────────────────────────────────────────────

function RootLayout(): JSX.Element {
  return (
    <div
      className="min-h-screen"
      style={{ background: "var(--ag-bg)", color: "var(--ag-text)" }}
    >
      <header
        className="sticky top-0 z-30 border-b backdrop-blur"
        style={{
          borderColor: "var(--ag-border)",
          background: "color-mix(in srgb, var(--ag-surface) 92%, transparent)",
        }}
      >
        <div className="mx-auto flex max-w-[1280px] items-center gap-3 px-4 py-3 sm:gap-6">
          <Link
            to="/"
            className="flex items-center gap-2 no-underline"
            style={{ color: "var(--ag-text)" }}
          >
            <span
              className="inline-flex h-7 w-7 items-center justify-center rounded-md text-sm font-bold text-white"
              style={{ background: "var(--ag-brand)" }}
              aria-hidden
            >
              a
            </span>
            <span className="text-sm font-semibold tracking-tight">
              algorithco guard
            </span>
            <span
              className="hidden rounded-full border px-1.5 py-0.5 text-xs sm:inline"
              style={{
                borderColor: "var(--ag-border)",
                color: "var(--ag-text-muted)",
              }}
            >
              algo
            </span>
          </Link>

          <nav className="flex items-center gap-1 text-sm" aria-label="Primary">
            <Link
              to="/"
              className="rounded-md px-2.5 py-1.5 hover:opacity-90 [&.active]:font-semibold"
              activeProps={{
                className: "active rounded-md px-2.5 py-1.5 font-semibold",
              }}
              style={{ color: "var(--ag-text)" }}
              activeOptions={{ exact: true }}
            >
              History
            </Link>
            <Link
              to="/audit"
              className="rounded-md px-2.5 py-1.5 hover:opacity-90 [&.active]:font-semibold"
              activeProps={{
                className: "active rounded-md px-2.5 py-1.5 font-semibold",
              }}
              style={{ color: "var(--ag-text)" }}
            >
              Audit
            </Link>
            <Link
              to="/policy"
              className="rounded-md px-2.5 py-1.5 hover:opacity-90 [&.active]:font-semibold"
              activeProps={{
                className: "active rounded-md px-2.5 py-1.5 font-semibold",
              }}
              style={{ color: "var(--ag-text)" }}
            >
              Policy
            </Link>
            <Link
              to="/stats"
              className="rounded-md px-2.5 py-1.5 hover:opacity-90 [&.active]:font-semibold"
              activeProps={{
                className: "active rounded-md px-2.5 py-1.5 font-semibold",
              }}
              style={{ color: "var(--ag-text)" }}
            >
              Stats
            </Link>
          </nav>

          <div className="ml-auto flex items-center gap-2">
            <a
              href="https://github.com/anomalyco/opencode"
              target="_blank"
              rel="noreferrer"
              className="hidden text-xs underline decoration-dotted sm:inline"
              style={{ color: "var(--ag-text-muted)" }}
            >
              docs
            </a>
            <ThemeToggle />
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-[1280px] px-4 py-6">
        <Outlet />
      </main>

      <footer
        className="border-t px-4 py-6"
        style={{
          borderColor: "var(--ag-border)",
          color: "var(--ag-text-muted)",
        }}
      >
        <div className="mx-auto max-w-[1280px] text-xs leading-relaxed">
          <p>
            <strong style={{ color: "var(--ag-text)" }}>
              algorithco guard
            </strong>{" "}
            — local-first, redact-before-network.{" "}
            <code
              className="rounded px-1 py-0.5"
              style={{
                background: "var(--ag-bg)",
                border: "1px solid var(--ag-border)",
              }}
            >
              algo init
            </code>{" "}
            is <code>local-only</code> by default (no data leaves the machine).
            BYOK <code>redacted</code> / <code>full</code> require explicit
            consent + <code>ALGO_JEV_API_KEY</code>. Inspect egress with{" "}
            <code>algo log --show-egress</code>.
          </p>
          <p className="mt-1">
            Tokens: Variant 1 single source{" "}
            <code style={{ color: "var(--ag-brand)" }}>
              plans/design-tokens.md
            </code>{" "}
            · Colors mapped via <code>design-tokens.css</code> →{" "}
            <code>Tokens.css</code> → Tailwind/shadcn. Decision badges use only{" "}
            <span style={{ color: "var(--ag-allow)" }}>allow</span> /{" "}
            <span style={{ color: "var(--ag-ask)" }}>ask</span> /{" "}
            <span style={{ color: "var(--ag-deny)" }}>deny</span>.
          </p>
        </div>
      </footer>
    </div>
  );
}

// ─── Route components ────────────────────────────────────────────────────────

function HistoryPage(): JSX.Element {
  const q = useQuery({
    queryKey: ["audit", { limit: 50 }],
    queryFn: () => fetchAuditHistoryWithFallback({ limit: 50 }),
  });

  const entries = q.data ?? [];

  return (
    <div className="grid gap-6">
      <div>
        <h1 className="text-xl font-semibold tracking-tight">History</h1>
        <p className="mt-1 text-sm" style={{ color: "var(--ag-text-muted)" }}>
          Every decision shows{" "}
          <strong style={{ color: "var(--ag-text)" }}>
            action + reason + confidence + source + latency
          </strong>{" "}
          — same as{" "}
          <code
            className="rounded px-1 py-0.5"
            style={{
              background: "var(--ag-surface)",
              border: "1px solid var(--ag-border)",
            }}
          >
            algo why
          </code>
          . Links below open the <code>algo why --trace</code> trace.
        </p>
      </div>

      <LiveFeed />

      <div className="grid gap-3">
        <h2 className="text-sm font-semibold">Recent decisions</h2>
        <HistoryTable data={entries} isLoading={q.isLoading} />
      </div>

      <div
        className="rounded-xl border border-dashed p-4"
        style={{
          borderColor: "var(--ag-border)",
          background: "color-mix(in srgb, var(--ag-surface) 60%, var(--ag-bg))",
        }}
      >
        <h3 className="text-sm font-semibold">
          Security findings — P4 placeholder
        </h3>
        <p className="mt-1 text-xs" style={{ color: "var(--ag-text-muted)" }}>
          Findings (repo-scan + scanner) will appear here when available. See{" "}
          <Link
            to="/stats"
            className="underline decoration-dotted"
            style={{ color: "var(--ag-brand)" }}
          >
            Stats → Security findings
          </Link>{" "}
          for the full placeholder. Static SPA shows this until backend provides
          audit findings.
        </p>
      </div>
    </div>
  );
}

function AuditPage(): JSX.Element {
  const q = useQuery({
    queryKey: ["audit", { limit: 200 }],
    queryFn: () => fetchAuditHistoryWithFallback({ limit: 200 }),
  });
  return (
    <div className="grid gap-6">
      <div>
        <h1 className="text-xl font-semibold">Audit log</h1>
        <p className="mt-1 text-sm" style={{ color: "var(--ag-text-muted)" }}>
          Local audit (SQLite <code>~/.algo/audit.db</code>, WAL) mirrors what
          daemon records. Redacted payloads only — inspect with{" "}
          <code
            className="rounded px-1 py-0.5"
            style={{
              background: "var(--ag-surface)",
              border: "1px solid var(--ag-border)",
            }}
          >
            algo log --show-egress
          </code>
          . Backend sync (when enabled) carries redacted only.
        </p>
      </div>
      <HistoryTable data={q.data ?? []} isLoading={q.isLoading} />
    </div>
  );
}

function PolicyPage(): JSX.Element {
  return (
    <div className="grid gap-6">
      <div>
        <h1 className="text-xl font-semibold">Policy</h1>
        <p className="mt-1 text-sm" style={{ color: "var(--ag-text-muted)" }}>
          Edit YAML,{" "}
          <strong style={{ color: "var(--ag-text)" }}>
            dry-run vs history
          </strong>{" "}
          (replays redacted audit), then publish. Daemon enforces rollback
          protection + signature.
        </p>
      </div>
      <PolicyEditor />
    </div>
  );
}

function StatsPage(): JSX.Element {
  return (
    <div className="grid gap-6">
      <div>
        <h1 className="text-xl font-semibold">Stats</h1>
        <p className="mt-1 text-sm" style={{ color: "var(--ag-text-muted)" }}>
          Per-user / per-project · time &amp; cost savings · blocked/asked
          breakdown · security findings placeholder (P4).
        </p>
      </div>
      <StatsView />
    </div>
  );
}

// ─── Router ──────────────────────────────────────────────────────────────────

const rootRoute = createRootRoute({ component: RootLayout });
const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: HistoryPage,
});
const auditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/audit",
  component: AuditPage,
});
const policyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/policy",
  component: PolicyPage,
});
const statsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/stats",
  component: StatsPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  auditRoute,
  policyRoute,
  statsRoute,
]);
const router = createRouter({ routeTree });

// ─── App ─────────────────────────────────────────────────────────────────────

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

export default function App(): JSX.Element {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
