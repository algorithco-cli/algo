# GitHub Org Plan — `algorithcoguard` (MONOREPO)

> Org: [`algorithcoguard`](https://github.com/algorithcoguard). Product: **algorithco guard**, CLI `algo`.
> Canonical repo: [`algorithcoguard/algorithco-guard`](https://github.com/algorithcoguard/algorithco-guard) — **single monorepo**.
> Status (2026-09-20): only `algorithcoguard/.github` (org profile + community health) is verified
> to exist via `gh repo list algorithcoguard`. The monorepo itself is **PLANNED — not yet created**.
> This plan **supersedes** the earlier multi-repo draft (8 separate repos: `proto`, `core`, `agent`,
> `backend`, `dashboard`, `web`, `eval`, `docs`). Do NOT create separate repos.
> Do NOT create the monorepo without human confirmation. `gh` commands below are documented, NOT executed.

## 1. Single repo + directory map

| Path | Language | Purpose |
|---|---|---|
| `docs/` | Markdown | ADRs, threat/security model, roadmap, tracking issues, this plan. |
| `proto/` | Protobuf + buf | Contracts: events, decisions, hook↔daemon, daemon↔backend, dataset schema. Tags generate Rust+TS. |
| `eval/` | Python | Datasets, harness, thresholds, distillation. Most important in Phase 0. Datasets may stay private as a submodule/split later — no split in Phase 0. |
| `core/` | Rust (`algo-*`) | Shared libs: types, shell-analysis, policy, redact, provider, fingerprint. |
| `agent/` | Rust | Hook client, daemon, adapters, CLI (`algo`), TUI, verifier, loop, scanner, MCP. Signed releases + SBOM. |
| `backend/` | Rust | Auth, orgs, signed policy sync, opt-in audit ingest, GitHub App. Phase 3 only until then stub. |
| `dashboard/` | TS React+Vite | Team app: history, findings, policy editor, stats. Static SPA. Phase 3 only until then stub. |
| `web/` | TS Astro+Starlight | Marketing + docs + install script hosting. Phase 2 (P2-08). |
| `plans/` | Markdown | Build-order plans (upstream of `docs/` exit gate). |
| `.github/` | YAML | CI, PR template, branch protection wiring. No CODEOWNERS copy here (see §2). |

Visibility is per-repo (the monorepo is one visibility unit). License ADR (D-B/D-07) + legal
sign-off decide public vs private at monorepo creation. Default bias: public once license clears
(trust for a security tool); keep private until then because `eval/` datasets and future
`backend/` user data live in the same repo. No LICENSE file is added until the license ADR
is accepted with legal sign-off — that decision is explicitly deferred, not made here.

Each top-level dir keeps its own `AGENTS.md` (build/test commands, conventions, human-review paths).

## 2. Per-directory ownership + CODEOWNERS enforcement

| Directory | Purpose owner | Security-critical paths (human review mandatory) |
|---|---|---|
| `docs/` | @algorithcoguard/maintainers | deny list, thresholds, redaction, sig-verify, `algo init`/`algo uninstall`, auth — @algorithcoguard/security (+ @algorithcoguard/privacy, @algorithcoguard/release, @algorithcoguard/backend, @algorithcoguard/agent, @algorithcoguard/core per path) |
| `proto/` | @algorithcoguard/core | breaking contract changes — @algorithcoguard/security + @algorithcoguard/core |
| `eval/` | @algorithcoguard/eval | gate thresholds, hard deny list tuning, dataset redaction — @algorithcoguard/security |
| `core/` | @algorithcoguard/core | deny list, thresholds, redaction, sig-verify — @algorithcoguard/security |
| `agent/` | @algorithcoguard/agent | adapters, daemon IPC, `algo init`/`algo uninstall`, sig-verify — @algorithcoguard/security |
| `backend/` | @algorithcoguard/backend | auth (device flow, tokens, orgs/roles), policy signing — @algorithcoguard/security |
| `dashboard/` | @algorithcoguard/maintainers | policy editor, auth wiring — @algorithcoguard/security + @algorithcoguard/backend |
| `web/` | @algorithcoguard/maintainers | install script hosting — @algorithcoguard/security + @algorithcoguard/release |

### CODEOWNERS location (verified 2026-09-20)

- Canonical file: `docs/CODEOWNERS`. GitHub natively recognizes CODEOWNERS in repository
> root, `.github/`, AND `docs/` — so `docs/CODEOWNERS` **IS enforced**; no action needed.
- Verified 2026-09-20: no `CODEOWNERS` copy exists at repo root or under `.github/`
> (checked via `**/CODEOWNERS` glob — single hit: `docs/CODEOWNERS`; `.github/` contains
> only `PULL_REQUEST_TEMPLATE.md` + `workflows/`).
- Rule: do NOT duplicate the file. Keep `docs/CODEOWNERS` as the single canonical copy.
> If a root or `.github/` copy ever appears, remove the duplicate and keep `docs/` canonical.
- One-repo-per-PR still applies at directory granularity: one top-level dir per PR
> (plus `docs/` linkage), so CODEOWNERS reviewers stay scoped.

## 3. Branch protection (enforce at monorepo creation)

- Default branch `main`. Require PR + passing CI (fmt/lint/type, tests, relevant gates per §4) before merge.
- **CODEOWNERS required reviewers** (agent approval never sufficient) on:
  - hard deny list + auto-approve threshold logic
  - redaction rules
  - signature verification (policy bundles, releases, self-update)
  - `algo init` / `algo uninstall` file-modification paths
  - auth (device flow, tokens, orgs/roles)
- Path-scoped required checks (same repo, scoped by path):
  - `proto/**`: require `buf lint` + `buf breaking` green on every PR touching contracts.
  - `core/**`, `agent/**`: require latency benchmarks + eval false-allow gate green.
  - `docs/adr/**`: ADR rule — CI blocks any `D-*` implementation PR without a merged ADR in `docs/adr/`.

## 4. Release flow (monorepo tags, consumers-pin-exact)

1. Contract change → PR touching `proto/` (with `buf breaking` pass) → merge to `main` → **tag** (semver on the monorepo, e.g. `v0.x.y`).
2. Tag drives generated Rust + TS packages from `proto/` (generate at build, never commit hand-edited output).
3. Consumers (`core/` → `agent/`/`backend/` → `dashboard`, `eval/` schemas) update in follow-up
   PRs, each recording the exact monorepo tag/SHA it was generated against.
4. Cross-directory sequence tracked by a single `docs/` tracking issue: `docs` issue → `proto/` PR → tag → consumer PRs.
5. Never commit hand-edited generated code; never duplicate proto-owned types by hand.

`core` distribution: git tags on the monorepo by default until the registry ADR (D-D / ADR-0005) says otherwise.

## 5. `gh` commands (DOCUMENTED ONLY — do not run without confirmation)

```powershell
# 0. Confirm auth (read-only, safe)
gh auth status

# 1. Re-check what already exists (read-only, safe)
gh repo list algorithcoguard --limit 50
gh repo view algorithcoguard/algorithco-guard

# 2. Create the monorepo (DESTRUCTIVE-ADJACENT — needs human OK; NOT executed in Phase 0)
# gh repo create algorithcoguard/algorithco-guard --private --description "algorithco guard: monorepo (algo CLI, core, agent, backend, dashboard, web, proto, eval, docs)"
# Visibility flips to public only after license ADR (D-B/D-07) + legal sign-off.

# 3. After creation: set defaults, branch protection, required checks per §3
#    (monorepo-wide protection + path-scoped checks; single docs/CODEOWNERS enforced — no copy needed)
```

Why not created now: Phase 0 gate needs license ADR (D-B/D-07, legal sign-off pending —
no LICENSE file until then), name ADR is DECIDED but repo visibility still needs legal
sign-off, and the human must confirm creation. Scaffold lives locally until then (see §6).

## 6. Local scaffold dirs (this workspace — Phase 0 stubs)

Created empty with `.gitkeep` + `README.md` stub each. **No product code in Phase 0.**

- `proto/` — contracts workspace (buf). Pinned tags generate Rust+TS at build.
- `eval/` — Phase 0 focus: datasets, harness, Jev measurement. Only throwaway redacted probe client in `eval/jev_client/`.
- `core/` — will hold `algo-*` crates in Phase 1 (types → fingerprint → shell → policy → redact → provider). Now: stub.
- `agent/` — will hold hook client, daemon, Claude-shell adapter, CLI/audit + shadow in Phase 1. Now: stub.
- `backend/` — Phase 3 only. Now: stub.
- `dashboard/` — Phase 3 only (Variant 1 tokens via `design-tokens.css`). Now: stub.
- `web/` — Phase 2 (P2-08) landing + docs (Starlight per `plans/design-tokens.md` §4). Now: stub.

Verify: `Get-ChildItem proto,eval,core,agent,backend,dashboard,web` shows only `.gitkeep` + `README.md` (+ `AGENTS.md` where present) per dir.

## 7. Tracking

Single-issue-per-workstream in `docs/tracking-issues.md` (monorepo issues, path-scoped PRs
linking `proto/` → `eval/` → `core/`/`agent/` order; see `plans/phase-0-00-overview.md` owners).
`infra` out of scope. Human sign-off required: threat model, ToS verdict, gate thresholds
(`docs/exit-gate-P0.md` §5).

## 8. Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/maintainers` + `@algorithcoguard/security` (+ `@algorithcoguard/legal` for visibility flip) — required before monorepo creation, before any visibility change, and before branch-protection/CODEOWNERS wiring changes. Without it, no repo creation, no public release, and no protection-rule edits.
