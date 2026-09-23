export interface Stat {
  value: number;
  prefix?: string;
  suffix?: string;
  label: string;
  icon: string;
}

export const STATS: Stat[] = [
  {
    value: 3,
    prefix: "<",
    suffix: "ms",
    label: "local decision p50",
    icon: "zap",
  },
  { value: 13, suffix: "", label: "versioned hard-deny rules", icon: "shield" },
  {
    value: 240,
    suffix: "",
    label: "eval records, plague-tested",
    icon: "flask",
  },
  {
    value: 174,
    suffix: "+",
    label: "tests green across the stack",
    icon: "check",
  },
];

export interface RoadmapItem {
  phase: string;
  title: string;
  body: string;
  state: "done" | "next" | "planned";
  icon: string;
}

export const ROADMAP: RoadmapItem[] = [
  {
    phase: "P1 — done",
    title: "Local MVP",
    body: "Daemon, hooks, Claude shell adapter, shadow mode, audit + CLI, quality gates green.",
    state: "done",
    icon: "check",
  },
  {
    phase: "P2 — next",
    title: "Enforcement",
    body: "Explicit enforce switch, verifier loop, edit/write guard, web policy review.",
    state: "next",
    icon: "rocket",
  },
  {
    phase: "P3 — planned",
    title: "Cloud + teams",
    body: "Dashboard, signed policy sync, org roles, SSO — the Team and Enterprise tiers.",
    state: "planned",
    icon: "cloud",
  },
  {
    phase: "P4 — planned",
    title: "Expansion",
    body: "Codex / OpenCode adapters, command scanner, TUI, GitHub app, Windows hardening.",
    state: "planned",
    icon: "grid",
  },
];

export interface PipelineStage {
  title: string;
  sub: string;
  body: string;
  accent?: boolean;
}

export const PIPELINE: PipelineStage[] = [
  {
    title: "Hook",
    sub: "fires first",
    body: "Your agent's hook sends every tool call to the tiny client (~1ms). Daemon down? It asks — exit 0, never blocks you.",
  },
  {
    title: "Parse + Redact",
    sub: "on-machine",
    body: "Tree-sitter parses the shell, the redactor masks secrets, fingerprinting normalizes the command for cache lookup.",
  },
  {
    title: "Decide",
    sub: "L0 / L1 / L3 / L4",
    body: "Hard-deny → cache (24h) → Jev judgment only if you opted in → ask. Source + latency attached; errors resolve to ask.",
    accent: true,
  },
  {
    title: "Render + Audit",
    sub: "logged",
    body: "The adapter replies approve / block / ask. Shadow logs would-have-blocked. Everything lands in ~/.algo/audit.db.",
  },
];

export const FAQS: Array<[string, string]> = [
  [
    "Does it block my commands today?",
    "Not unless you ask it to. Guard ships shadow-first: it records what it would have blocked (would-have N digest) and never blocks until you run algo enforce on.",
  ],
  [
    "Which agents are supported?",
    "Claude Code Bash hooks today (shell-only adapter). Codex and OpenCode adapters plus a scanner are Phase 4. The daemon speaks a versioned proto contract, so adapters stay thin.",
  ],
  [
    "Does my code leave my machine?",
    "No — fresh algo init is local-only and never sends anything. Only the explicit redacted (BYOK) or full modes send data, always to TypeSafe AI's US infrastructure under your own key, after on-machine redaction. Inspect anything with algo log --show-egress.",
  ],
  [
    "How fast is it?",
    "Local decisions (policy + cache) target p50 <3ms / p99 <10ms, enforced in CI. The hook client starts in ~1ms. Jev judgments (opt-in only) budget p50 <250ms.",
  ],
  [
    "What does it cost?",
    "The local MVP is free. Team and Enterprise tiers are planned for the Phase 3 cloud (dashboard, policy sync, SSO/SCIM) — pricing below is what we're building toward.",
  ],
  [
    "Is it open source?",
    "The monorepo stays private until release. Decisions, thresholds, and the P0 eval dataset are versioned internally and gated by human review.",
  ],
];

export const CLI_ROWS: Array<[string, string]> = [
  ["algo init", "Detect agents → diff → consent → hooks → privacy (~30s)"],
  ["algo doctor", "Verify install, daemon, socket, DB, configs"],
  ["algo status", "Counts, savings, profile + threshold source"],
  ["algo why", "action + reason + confidence + source + latency"],
  ["algo log", "Local audit log; --show-egress inspects egress"],
  ["algo enforce [on|off]", "Shadow → enforce switch (explicit opt-in)"],
  ["algo pause / resume", "One-step stop; works daemon-dead"],
  ["algo login", "OAuth device flow for team/cloud (P3)"],
  ["algo policy", "View / dry-run policy bundles"],
];
