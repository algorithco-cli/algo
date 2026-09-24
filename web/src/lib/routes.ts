/* web — single route manifest (IA source of truth).
 * Marketing + docs endpoints for the private MVP static site.
 * Sidebar order, prev/next, SEO, and sitemap/prerender all derive from here.
 * Hash anchors (#install etc.) are preserved as in-page ids for deep links.
 */

export interface RouteMeta {
  path: string;
  title: string;
  description: string;
  layout: "marketing" | "docs" | "minimal";
  indexable: boolean;
  prev?: string;
  next?: string;
}

export const ROUTES: readonly RouteMeta[] = [
  {
    path: "/",
    title: "Algorithco Guard — safer AI coding agents",
    description:
      "Control layer for AI coding agents. Hard-deny destructive commands, fail-safe ask, redact-first privacy. Free local MVP, ~30s install.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/features",
    title: "Features — safer, quieter, auditable guardrails | Algorithco Guard",
    description:
      "13 hard-deny rules, fail-safe ask on every error, shadow-first rollout, algo why audit. Under 3ms local decisions.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/how",
    title: "How it works — hook to audit in milliseconds | Algorithco Guard",
    description:
      "Hook client, on-machine parse+redact, L0/L1/L3 decision, SQLite audit. Fail-safe ask, redacted egress inspector.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/pricing",
    title: "Pricing — free local, team + enterprise planned | Algorithco Guard",
    description:
      "Local MVP free forever. Team $19/seat/mo and Enterprise custom planned for Phase 3 cloud with dashboard and SSO.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/roadmap",
    title: "Roadmap — local MVP to cloud + expansion | Algorithco Guard",
    description:
      "P1 local done, P2 enforcement next, P3 cloud+teams, P4 Codex/OpenCode, scanner, TUI, Windows. Gates per phase.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/login",
    title: "Log in — email or provider sign-in | Algorithco Guard",
    description:
      "Create an account with email or sign in with GitHub/Google. Team cloud with org policy and dashboard is Phase 3 planned; local MVP runs today.",
    layout: "marketing",
    indexable: false,
  },
  {
    path: "/billing",
    title: "Billing — manage subscription and seats | Algorithco Guard",
    description:
      "Manage your org subscription: pick Pro, Max, or Team per seat, monthly or annual, update seats, or cancel at period end.",
    layout: "marketing",
    indexable: false,
  },
  {
    path: "/faq",
    title: "FAQ — blocking, agents, privacy, speed, cost | Algorithco Guard",
    description:
      "Does Guard block commands? Which agents? Does code leave? How fast? Cost? Open source? Shadow-first answers.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/privacy",
    title:
      "Privacy — local-only by default, BYOK redacted opt-in | Algorithco Guard",
    description:
      "local-only sends nothing. BYOK redacted/full sends to TypeSafe US infra, no training on Input, as-long-as-necessary retention. Inspect with algo log.",
    layout: "marketing",
    indexable: true,
  },
  {
    path: "/docs",
    title: "Docs — install once, know every command | Algorithco Guard",
    description:
      "One binary, nine commands. Additive install, byte-identical uninstall. Start with install, then CLI reference.",
    layout: "docs",
    indexable: true,
    next: "/docs/install",
  },
  {
    path: "/docs/install",
    title: "Install — algo init in ~30s (Claude Code now) | Algorithco Guard",
    description:
      "curl install.sh, algo init diff+consent, doctor verify. Codex/OpenCode adapters Phase 4. Verify-before-run included.",
    layout: "docs",
    indexable: true,
    prev: "/docs",
    next: "/docs/cli",
  },
  {
    path: "/docs/cli",
    title:
      "CLI reference — algo init/doctor/status/why/log/enforce | Algorithco Guard",
    description:
      "Nine commands: init, doctor, status, why (action+reason+confidence+source+latency), log --show-egress, enforce, pause, login, policy.",
    layout: "docs",
    indexable: true,
    prev: "/docs/install",
  },
];

export const INDEXABLE_PATHS: readonly string[] = ROUTES.filter(
  (r) => r.indexable,
).map((r) => r.path);

export function routeMeta(path: string): RouteMeta | undefined {
  return ROUTES.find((r) => r.path === path);
}

/** Header nav (marketing top-level). */
export const NAV_LINKS: readonly { label: string; to: string }[] = [
  { label: "Features", to: "/features" },
  { label: "How it works", to: "/how" },
  { label: "Docs", to: "/docs" },
  { label: "Pricing", to: "/pricing" },
];

/** Docs sidebar — styled to match branched menu Image 2 (Getting started / Components pattern). */
export const DOCS_SIDEBAR: readonly {
  label: string;
  children: { value: string; label: string; to: string }[];
}[] = [
  {
    label: "Getting started",
    children: [
      { value: "docs", label: "Overview", to: "/docs" },
      { value: "install", label: "Install", to: "/docs/install" },
    ],
  },
  {
    label: "Reference",
    children: [{ value: "cli", label: "CLI reference", to: "/docs/cli" }],
  },
  {
    label: "Trust",
    children: [{ value: "privacy", label: "Privacy modes", to: "/privacy" }],
  },
];
