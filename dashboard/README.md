# dashboard — algorithco guard team SPA

> **Stack:** React 18 + Vite 5 static SPA · TanStack Router / Query / Table · Tailwind 3 + shadcn/ui · uPlot · SSE live feed · API client stub from proto `algorithco_guard.v0`.
> **No Node runtime in prod** — `dist/` is static and deploys to any static host.
> **Colors:** single source `design-tokens.md` Variant 1 DECIDED — `design-tokens.css` → `src/components/Tokens.css` → Tailwind/shadcn mapping. No hard-coded hex outside tokens.

## Quick start

```powershell
# from repo root
cd dashboard
npm install
npm run dev      # http://localhost:5173 (proxies /v1 to http://localhost:8080)
npm run build    # static dist/ — no server JS
npm run lint     # tsc --noEmit
npm run preview  # preview dist/ at http://localhost:4173
```

## Structure

```
dashboard/
  design-tokens.css            # canonical tokens (copy of ../design-tokens.css)
  tailwind.config.js           # brand/bg/surface/border/text + allow/ask/deny → tailwind + shadcn aliases
  vite.config.ts               # proxies /v1 → local daemon/backend if present
  index.html
  src/
    main.tsx                   # mounts App, imports Tokens.css + index.css + uPlot css
    App.tsx                    # TanStack Router: / (History), /audit, /policy, /stats + TanStack Query
    index.css                  # tailwind base/components/utilities + token-aware resets
    components/
      Tokens.css               # @import design-tokens.css + @theme inline + shadcn var mapping
      HistoryTable.tsx         # TanStack Table: ts, tool, action badge (allow/ask/deny), reason, confidence, source, latency, why link
      PolicyEditor.tsx         # YAML textarea + Dry-run vs history + Publish (mock when no backend)
      LiveFeed.tsx             # EventSource /v1/audit/stream → polling fallback every 5s
      StatsView.tsx            # allow/ask/deny breakdown, uPlot 14-day trend, per-user/project, savings, P4 findings placeholder
      ui/badge.tsx, button.tsx, card.tsx
    lib/
      api.ts                   # GuardService stub mirroring proto/algorithco_guard/v0/backend.proto + decision/events
      mock.ts                  # fallback audit + stats + per-user/project (static SPA demo)
      utils.ts
```

## API — proto contract

`src/lib/api.ts` is a **generated-ish stub** for `GuardService` (ConnectRPC). Canonical protos:

- `proto/algorithco_guard/v0/backend.proto` — `Org/Team/Role, PolicyBundle, GetPolicy/PublishPolicy/DryRun, IngestAudit, QueryStats`
- `proto/algorithco_guard/v0/decision.proto` — `Action, SourceLevel, Decision`
- `proto/algorithco_guard/v0/events.proto` — `ToolKind, PrivacyMode, ToolBefore/After`

Endpoints mapped for the static SPA (REST shim):

| Method | Path | Proto RPC |
|--------|------|-----------|
| GET | `/v1/audit?limit=&org_id=` | list audit (daemon or backend) |
| POST | `/v1/audit` | `IngestAudit` |
| GET | `/v1/audit/stream` | SSE live feed (EventSource) |
| GET | `/v1/policy?version=&org_id=` | `GetPolicy` |
| POST | `/v1/policy` | `PublishPolicy` |
| POST | `/v1/policy/dryRun` | `DryRun` |
| GET | `/v1/stats?org_id=&from=&to=` | `QueryStats` |

All fetchers have `*WithFallback` variants that return mock data when the backend is absent — so `npm run build` + static host still renders a useful demo and Playwright can run without a live backend. Replace with the buf-generated TS client at the `P3-01` tag (consumers pin exact proto version per `AGENTS.md` contracts-first).

Env:

- `VITE_API_BASE` — override API base (default `""` → relative, dev proxy handles `/v1`).

## Design tokens

Single source: `plans/design-tokens.md` (Variant 1). Canonical CSS is `design-tokens.css`:

```css
:root,[data-theme="light"]{ --ag-brand:#6D4AFF; --ag-bg:#FAFAFB; --ag-surface:#FFFFFF; --ag-border:#E6E5EE; --ag-text:#17161F; --ag-text-muted:#6B6A7B; --ag-allow:#1E9E63; --ag-ask:#D99A00; --ag-deny:#E5484D; }
[data-theme="dark"]{ --ag-brand:#8E77FF; --ag-bg:#0E0D15; --ag-surface:#16151F; --ag-border:#26243A; --ag-text:#F4F3FF; --ag-text-muted:#9C9AB0; --ag-allow:#3DD68C; --ag-ask:#F5B82E; --ag-deny:#FF6B6F; }
```

`src/components/Tokens.css` imports that file, adds `@theme inline` (Tailwind v4 compat) and shadcn mappings (`primary → brand`, `background → bg`, `card → surface`, etc.). CI enforces `grep -ri '#[0-9a-f]\{6\}' dashboard/src` returns only that file.

## Views

- **History (`/`)** — table with `action` badge colored `allow/ask/deny`, reason, confidence, source, latency, and `algo why --trace <id>` link (action+reason+confidence+source+latency). SSE live feed above the table (shares query with TUI/status).
- **Audit (`/audit`)** — full audit log (same table, limit 200, local SQLite `~/.algo/audit.db` when live). Help text points to `algo log --show-egress`.
- **Policy (`/policy`)** — textarea for policy YAML (versioned bundle, detached sig), **Dry-run vs history** (replays bundle vs redacted audit), **Publish** (mock in static build; backend verifies `sig` + monotonic version live).
- **Stats (`/stats`)** — blocked/asked breakdown, 14-day trend (uPlot), per-user / per-project tables, time/cost savings estimate (static placeholder; prod uses backend `QueryStats`), security findings placeholder (P4).

## Fail-safe & privacy

- Any I/O / timeout / parse error resolves to `ask` (never `allow`) — `proves_ask_on_*` in the table and dry-run paths.
- Static fallback mocks never claim vendor measurements; savings copy is labeled “static estimate — no vendor claim without measurement link” per `AGENTS.md` §8.
- Privacy copy in footer matches `docs/privacy-dataflow.md` (local-only default, BYOK redacted/full only with consent, `algo log --show-egress`, US-hosted when enabled).

## Verification (static check without backend)

```powershell
cd dashboard
npm run lint   # tsc --noEmit — should be clean
npm run build  # vite build → dist/
```

`dist/` is the deploy artifact (static, no server JS). `npm audit` should be clean after `npm install` with the pinned versions above.

## No backend? (MVP static)

The SPA is usable with no daemon/backend: queries fall back to `src/lib/mock.ts`. Live feed falls back from SSE to polling `GET /v1/audit`. To connect a live daemon/backend, run it on `http://localhost:8080` (or set `VITE_API_BASE`).

## Deferrals

P4 scanner findings are a dashed placeholder in `/` and `/stats`. Real findings will land via the backend audit stream per `plans/phase-4-scanner-questions.md`.
