# web — algorithco guard docs site

> **Stack:** Vite 5.4.21 static · React 18.3.1 · `design-tokens.css` Variant 1 single source
> **Privacy text must match behavior** — no vendor claims without a measurement link.
> **Local URL:** `http://127.0.0.1:3007` (dev + preview, `strictPort`). No network on page load — GitHub stars load only on explicit click.

## Quick start

```powershell
cd web
npm install
npm run dev      # http://127.0.0.1:3007
npm run build    # static dist/
npm run lint     # tsc --noEmit + biome check
npm run test     # vitest run (verdict, hex-guard, sitemap/og)
npm run gen:deny # parity check core deny_list.rs vs src/lib/verdict.ts
npm run preview  # preview dist/ at http://127.0.0.1:3007 (NO vite proxy — billing needs a VITE_BACKEND_URL build + running backend, see below)
```

> **Preview/billing note:** `vite preview` serves `dist/` with no `/api` proxy.
> The billing page works under `npm run dev` (proxy `/api/*` → `127.0.0.1:8080`
> with the `/api` prefix stripped) or under preview/dist only when built with
> `VITE_BACKEND_URL` set to the backend's bare origin (e.g.
> `http://127.0.0.1:8080`, no trailing `/api`) and the backend running with
> CORS allowing the page origin (`ALGO_CORS_ORIGINS`).

## What this site is

- **Marketing + docs** for `algorithco guard` (CLI `algo`) — real endpoints, not hash anchors:
  `/` `/features` `/how` `/pricing` `/roadmap` `/login` `/faq` `/privacy` `/docs` `/docs/install`
  `/docs/cli` + `404`. Route manifest: `src/lib/routes.ts` (titles, descriptions, prev/next).
- **Privacy page** documents the modes:
  - Modes table (`local-only` default, `redacted` BYOK only-real-data, `full` second consent)
  - Where your data goes (TypeSafe AI, US-hosted, subprocessors AWS / Modal / Nebius / CoreWeave / Slack / Google Workspace — all USA)
  - Retention: “as long as reasonably necessary”, no fixed SLA
  - Training: will not train on Input, will not disclose Input except to service providers
  - ZDR enterprise-only via `privacy@typesafe.ai`
  - Dataflow diagram + fail-safe / latency / redact-before-network rules
- **Docs** documents `algo init` (~30s), `algo doctor`, `algo status`, `algo why`
  (action+reason+confidence+source+latency), `algo log --show-egress`, `algo enforce`,
  `algo pause`/`algo uninstall` across `/docs`, `/docs/install`, `/docs/cli` with breadcrumbs + prev/next.

No secrets, no invented APIs, no hard-coded hex outside `design-tokens.css` → `src/components/Tokens.css`. Variant 1 only — grep check: `grep -ri '#[0-9a-f]\{6\}' web/src` should return only the token files.

## Structure

```
web/
  design-tokens.css            # canonical tokens (copy of ../design-tokens.css)
  index.html                   # shell; per-route head injected by scripts/prerender.mjs
  vite.config.ts               # manualChunks react/vendor, lucide icons alias, warmup
  scripts/prerender.mjs        # post-build: dist/<route>/index.html + 404.html + sitemap.xml
  src/
    main.tsx                   # BrowserRouter shell
    App.tsx                    # router shell only (<AppRoutes/>)
    routes.tsx                 # lazy Routes (marketing + docs layouts)
    lib/routes.ts              # IA manifest: path/title/desc/layout/prev/next
    lib/site.ts                # SITE_URL + canonicalFor()
    components/layout          # RootLayout/Header/Footer + Marketing/Docs layouts + Seo/Breadcrumbs/PrevNext
    components/                # Section/Counter/CopyButton/VerdictDemo/PipelineDiagram/InstallTabs/DeviceLogin/PixelGuardDog/Dither/FaqList/Cta
    lib/auth.ts                # live auth client (email signup/login, GitHub device, Google OIDC, backend liveness)
    pages/                     # Home/Features/How/Pricing/Roadmap/Login/Faq/Privacy/NotFound
    pages/docs/                # DocsIndex/Install/Cli
    components/Tokens.css      # @import design-tokens.css + shadcn var mapping (web side)
    styles.css                 # base CSS using token vars (no Tailwind)
```

## Tokens

Single source `design-tokens.css` → `src/components/Tokens.css`. Dark is not an inversion — use exact `To'q mavzu` values. Decision colors exclusive to `allow/ask/deny`.

## Verification

```powershell
cd web
npm run lint      # tsc --noEmit + biome check
npm run test      # vitest run
npm run gen:deny  # core ↔ web deny-list parity (must be OK)
npm run build     # tsc -b && vite build && node scripts/prerender.mjs (11 routes + 404 + sitemap)
```

Static `dist/` deploys anywhere. `grep -ri '#[0-9a-f]\{6\}' src` should only hit `src/components/Tokens.css` (+ `src/lib/site.ts` has none).
Sitemap/og use absolute `http://127.0.0.1:3007` (private MVP local canonical, see `src/lib/site.ts`). Production domain TBD — update `site.ts` + `index.html` + `public/sitemap.xml` + `public/robots.txt` together.

## Relation to dashboard

- `dashboard` is the team SPA (history/policy/stats/SSE, `http://localhost:5173` in dev). This `web` site links to it.
- Both consume the same `design-tokens.css` single source. This Vite site maps `primary`/`--sl-color-accent` to `var(--ag-brand)` directly (Starlight alias kept for compat, no Starlight runtime).
