# Phase 3 — Dashboard SPA

> Stack: React+Vite static SPA, TanStack Router/Query/Table, Tailwind shadcn/ui, uPlot, SSE live feed. API client generated from proto. No Node runtime in prod.
> Design tokens: single source `design-tokens.md` (Variant 1 DECIDED). No local palette — import CSS vars, map shadcn primary/bg/card/border/foreground to brand/bg/surface/border/text, decision badges to allow/ask/deny tokens.

## Views

- History with explanations (links to `algo why`: action+reason+confidence+source+latency).
- Blocked/asked breakdown, security findings placeholder (P4 fills).
- Policy editor + dry-run vs history.
- Per-user/project stats, time/cost savings.
- SSE live decision feed (shares query with TUI/status).

## Build

Generate TS client from P3-01 tag. Static build only. Biome clean, `npm audit` clean, a11y basics.

## Acceptance

Playwright: login→history→policy edit→dry-run→publish→daemon picks up. Needs live backend (testcontainers or ephemeral). Static artifact deploys with no server JS.
