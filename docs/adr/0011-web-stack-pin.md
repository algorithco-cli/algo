# ADR-0011: web stack pin (React 18 + Vite 5) + upgrade plan

## Status

draft — 2026-09-22 (private MVP, `web/` only)

## Context

`web/` is a Vite static single-page docs/marketing site (`web/package.json:1-25`):
`react@^18.3.1`, `react-dom@^18.3.1`, `lucide-react@^1.47.0`, dev `vite@^5.4.8`
(resolves `5.4.21`), `@vitejs/plugin-react@^4.3.3` (resolves `4.7.0`),
`typescript@^5.5.4` (resolves `5.9.3`). Dev server now `http://127.0.0.1:3007`
(`web/vite.config.ts`), preview same port.

`npm outdated` (2026-09-22): all 7 behind a major — `react/react-dom 18.3.1 → 19.3.0`,
`vite 5.4.21 → 8.3.0`, `typescript 5.9.3 → 7.0.2`, `plugin-react 4.7.0 → 6.1.1`,
`@types/* → 19.x`. `npm audit` (2026-09-22): 2 vulns, fix requires breaking major:
HIGH `GHSA-fx2h-pf6j-xcff` (`server.fs.deny` bypass on Windows alternate paths,
CVSS 7.5) + 2 moderate (`.map` path traversal; `launch-editor` NTLMv2 UNC leak)
+ transitive `esbuild ≤0.24.2` moderate `GHSA-67mh-4wv8-2f99`. Dev-server-only
exposure for a static site, but Windows bypass is directly relevant to P4
Windows hardening (see ADR-0007). `AGENTS.md §2` dep-audit gate currently fails.

No router/store/Tailwind/SSR — single `App.tsx` hash-anchor SPA + local
`useState` only. Staying on React 18 + Vite 5 is a stability choice for the
private MVP, but it was silent. This ADR records the pin and the upgrade path.

## Decision

Pin `web/` for the private MVP and record the upgrade plan:

- Pin: `react`/`react-dom` `18.3.1`, `vite` `5.4.21`, `@vitejs/plugin-react` `4.7.0`,
  `typescript` `5.9.3`, `lucide-react` `1.47.0`. `package-lock.json` is source of
  truth. No React 19 / Vite 6+ / TS 7 upgrade during private MVP without a
  superseding ADR.
- Serve dev/preview on `127.0.0.1:3007` (`vite.config.ts: server/preview.port 3007`,
  `strictPort`). `README.md` documents `:3007`.
- Accept dev-server-only vuln exposure for local-only private MVP (no public
  hosting, no `vite preview --host 0.0.0.0`, no prod deploy from dev server).
- Upgrade plan (before public release, in order):
  1. `vite 5.4.21 → 8.3.0` (major) to clear `GHSA-fx2h-pf6j-xcff` + transitives.
     Expect breaking: new `server.fs` semantics, `launch-editor` removal/flag,
     `esbuild` bump, plugin-react `4.x → 6.x` compat, Node `≥20.19` check.
  2. `typescript 5.9 → 7.0` (major) + `tsconfig` re-baseline (`moduleResolution bundler`
     still, `verbatimModuleSyntax` check).
  3. `react 18 → 19` + `@types/* → 19.x` + `lucide-react` major check. Codemod
     ref/hydration/StrictMode regressions; re-run vitest + playwright smoke.
  4. Re-run `npm audit` clean, `npm outdated` review, `vite build` repro of `dist/`,
     Biome + vitest gates green.
- Add missing gates now (same PR batch): Biome + vitest smoke
  (`demoVerdict`, token-hex guard, sitemap/og absolute-URL check). See
  `web/README.md: Verification`.

## Alternatives

- **Upgrade to Vite 8 / React 19 now**: rejected for private MVP. Breaks plugin
  compat + needs full E2E re-verify while `web/` has zero tests. Correct after
  smoke gates land, before public hosting.
- **`npm audit fix --force` now**: rejected. Same breaking jump, silent, no ADR,
  no test cover to catch regressions.
- **Stay silent on old majors (no ADR)**: rejected. Violates ADR-before-code for a
  known HIGH + `AGENTS.md §2` dep-audit gate; hides Windows P4 risk.

## Consequences

### Positive

- Deterministic private-MVP builds on a known-good pin; `dist/` reproducible.
- HIGH advisory explicitly accepted with scope (local dev only) + dated upgrade
  path instead of silent debt.
- Windows P4 risk called out: dev-server bypass does not affect static `dist/`
  serving, but any public preview/hosting must wait for Vite 8.

### Negative

- Ships with known HIGH in devDeps until upgrade (mitigated: loopback only,
  `127.0.0.1`, no public preview).
- React 19 features unavailable; future upgrade is breaking + needs codemod time.
- `dist/` + `tsconfig.tsbuildinfo` stay committed for MVP repro (normally
  gitignored) — cleanup deferred to upgrade PR.

## Verification

- `cd web && npm install && npm run lint && npm run build` green on pin
  (`tsc --noEmit` + `tsc -b && vite build`, 2026-09-22).
- `npm audit` output archived in PR description (2 vulns, HIGH `GHSA-fx2h-pf6j-xcff`
  accepted for loopback-only MVP, upgrade tracked here).
- `npm outdated` output archived in PR description (7 majors behind, pinned).
- Dev: `http://127.0.0.1:3007/` returns `200` + `<title>Algorithco Guard`.
- Upgrade PR must show `npm audit` clean + `vite build` + Biome + vitest green.
- Official links (2026-09-22): https://github.com/advisories/GHSA-fx2h-pf6j-xcff,
  https://vite.dev/guide/migration, https://react.dev/blog/2024/04/25/react-19-upgrade-guide

## Update 2026-09-23 — upgrade step 1 pulled forward (supersedes the pin above)

CI (`Web / web`, PR #1) proved a **critical** advisory the 2026-09-22 audit
did not list: `vitest ≤4.1.10` "When Vitest UI server is listening, arbitrary
file can be read and executed" (fix: `vitest@5.0.1`, semver-major), plus
react-router highs. The "no Vite 6+ during private MVP" pin is therefore
superseded for `web/` by this update (still draft, unsigned — human merge
of the PR ratifies it):

- `vite ^5.4.8 → ^8.3.0`, `@vitejs/plugin-react ^4.3.3 → ^6.1.1`,
  `vitest ^2.1.9 → ^5.0.1` (vitest 5 peers `vite ^6.4 || ^7 || ^8`, so the
  critical fix requires the Vite major). Installed sequentially (joint
  `vite+plugin+vitest` install ERESOLVEs) + compatible `npm audit fix`
  (react-router-dom → 6.30.6).
- `vite.config.ts`: dropped `rollupOptions.output.manualChunks` object form
  (removed in Vite 8 — function-only now); chunking loss is perf tuning, not
  correctness.
- Verified 2026-09-23: `npm run lint` (tsc + biome, 61 files) clean,
  `vitest run` 28/28 passed, `vite build` + prerender (11 routes) green.
  `npm audit`: **0 critical, 0 high**, 2 moderate remaining
  (react-router pair — dev-server open-redirect scope, same loopback-only
  acceptance as before).
- `dashboard/` untouched (its audit gate passed: 1 moderate esbuild, dev-only).
- Remaining plan steps (TS 7, React 19) stay deferred to pre-public-release.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: __________ Date: __________
- [ ] Upgrade to Vite 8 approved (before public hosting): __________ Date: __________
