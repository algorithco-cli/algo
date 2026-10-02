# web — algorithco guard docs site

> **Stack:** Vite 8 · React 18 · Motion · `design-tokens.css` Variant 1 single source
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

Account-mode builds set `VITE_GUARD_AUTH_MODE=account` and the three required
account values shown in [`.env.example`](.env.example). `guard-web` is a public
OIDC client: the SPA uses authorization code + PKCE S256 and sends no client
secret. Its access, ID, and refresh tokens are memory-only; a page reload starts
a new top-level authorization redirect and relies on the account SSO session.
The configured redirect URI must exactly match a URI registered for the client.
Legacy builds continue to use the existing login and billing UI until PR6 flips
the default.

> **Preview/billing note:** `vite preview` serves `dist/` with no `/api` proxy.
> The billing page works under `npm run dev` (proxy `/api/*` → `127.0.0.1:8080`
> with the `/api` prefix stripped) or under preview/dist only when built with
> `VITE_BACKEND_URL` set to the backend's bare origin (e.g.
> `http://127.0.0.1:8080`, no trailing `/api`) and the backend running with
> CORS allowing the page origin (`ALGO_CORS_ORIGINS`).

## What this site is

- **Marketing + docs** for `algorithco guard` (CLI `algo`) — real endpoints, not hash anchors:
  `/` `/features` `/how` `/pricing` `/roadmap` `/login` `/billing` `/faq` `/privacy`
  `/terms` `/security` `/docs/*` + `404`. Route manifest: `src/lib/routes.ts`
  (titles, descriptions, docs order, and prev/next).
- **Team dashboard** (dashboard plan): `/dashboard`
  (overview: backend liveness + session), `/dashboard/history|audit|policy|stats`
  (ported views: react-table history with inline `algo why`, authed SSE-burst
  live feed, YAML policy dry-run/publish, uPlot stats with per-user tables).
  Client-only: excluded from prerender + sitemap (`indexable: false`), lazy
  chunks, React Query cache, unified client `src/lib/guard.ts` over the backend
  dashboard shims with labeled static demo fallback. Dark-only Variant 1.
- **Legal and trust pages** distinguish the local product from preview hosted features. Before hosted GA,
  counsel/ownership must supply the contracting entity, jurisdiction, verified contact addresses,
  processors, locations, and retention schedule.
- **Docs** documents `algo init` (~30s), `algo doctor`, `algo status`, `algo why`
  (action+reason+confidence+source+latency), `algo log --show-egress`, `algo enforce`,
  `algo pause`/`algo uninstall`, configuration, architecture, privacy/security, and troubleshooting.
- **Motion** uses the existing `motion` dependency, one-time viewport reveals, and
  `prefers-reduced-motion` support. Hero animation colors are read from CSS tokens.

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
    components/                # layouts, MotionReveal, legal/docs shells, diagrams, demos
    components/                # Section/Counter/CopyButton/VerdictDemo/PipelineDiagram/InstallTabs/DeviceLogin/PixelGuardDog/Dither/FaqList/Cta
    lib/auth.ts                # live auth client (email signup/login, GitHub device, Google OIDC, backend liveness)
    pages/                     # marketing, preview account, legal, security, and 404 pages
    pages/docs/                # overview, install, CLI, configuration, architecture, trust, help
    components/Tokens.css      # web token source + compatibility aliases
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
npm run build     # tsc -b && vite build && node scripts/prerender.mjs (18 routes + 404 + sitemap)
```

Static `dist/` deploys anywhere. `grep -ri '#[0-9a-f]\{6\}' src` should only hit `src/components/Tokens.css` (+ `src/lib/site.ts` has none).
Sitemap/og use absolute `http://127.0.0.1:3007` (private MVP local canonical, see `src/lib/site.ts`). Production domain TBD — update `site.ts` + `index.html` + `public/sitemap.xml` + `public/robots.txt` together.

## Relation to dashboard

- The standalone `dashboard/` SPA (history/policy/stats/SSE,
  `http://localhost:5173` in dev) is the frozen reference. Its views are
  ported here (dashboard plan, live-verified); do not add features there.
- Both consume the same `design-tokens.css` single source (dark-only enforced
  by `src/lib/site.test.ts`). This Vite site maps `primary`/`--sl-color-accent` to `var(--ag-brand)` directly (Starlight alias kept for compat, no Starlight runtime).
