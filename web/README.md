# web — algorithco guard docs site

> **Stack:** Vite 5 static · React 18 · `design-tokens.css` Variant 1 single source
> **Privacy text must match behavior** — source: `docs/privacy-dataflow.md` (owner-confirmed 2026-09-20). No vendor claims without a measurement link.

## Quick start

```powershell
cd web
npm install
npm run dev      # http://localhost:5174
npm run build    # static dist/
npm run lint     # tsc --noEmit
npm run preview  # preview dist/ at http://localhost:4174
```

## What this site is

- **Marketing + docs** for `algorithco guard` (CLI `algo`) — install, quickstart, privacy, dataflow.
- **Privacy page** embeds the exact copy from `docs/privacy-dataflow.md`:
  - Modes table (`local-only` default, `redacted` BYOK only-real-data, `full` second consent)
  - Where your data goes (TypeSafe AI, US-hosted, subprocessors AWS / Modal / Nebius / CoreWeave / Slack / Google Workspace — all USA)
  - Retention: “as long as reasonably necessary”, no fixed SLA
  - Training: will not train on Input, will not disclose Input except to service providers
  - ZDR enterprise-only via `privacy@typesafe.ai`
  - Consent text draft for `algo init` + footer privacy notice (both verbatim)
  - Dataflow diagram + fail-safe / latency / redact-before-network rules
- **Install** section documents `algo init` (~30s), `algo doctor`, `algo status`, `algo why` (action+reason+confidence+source+latency), `algo log --show-egress`, `algo enforce`, `algo pause`/`algo uninstall`.

No secrets, no invented APIs, no hard-coded hex outside `design-tokens.css` → `src/components/Tokens.css`. Variant 1 only — grep check: `grep -ri '#[0-9a-f]\{6\}' web/src` should return only the token files.

## Structure

```
web/
  design-tokens.css            # canonical tokens (copy of ../design-tokens.css)
  index.html
  vite.config.ts
  src/
    main.tsx
    App.tsx                    # single-page docs: hero, install, quickstart, privacy (verbatim), dataflow
    components/Tokens.css      # @import design-tokens.css + shadcn var mapping (web side)
    styles.css                 # base CSS using token vars (no Tailwind)
```

## Tokens

Single source `plans/design-tokens.md` → `design-tokens.css` → `src/components/Tokens.css`. Dark is not an inversion — use exact `To'q mavzu` values. Decision colors exclusive to `allow/ask/deny`.

## Verification

```powershell
cd web
npm run lint
npm run build
```

Static `dist/` deploys anywhere. `grep -ri '#[0-9a-f]\{6\}' src` should only hit token files.

## Relation to dashboard

- `dashboard` is the team SPA (history/policy/stats/SSE, `http://localhost:5173` in dev). This `web` site links to it.
- Both consume the same `design-tokens.css` single source per `plans/design-tokens.md` §4 (Starlight override `--sl-color-accent: var(--ag-brand)` when Starlight is used; this Vite site maps `primary` to `var(--ag-brand)` directly).
