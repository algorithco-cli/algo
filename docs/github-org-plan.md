# GitHub Org Plan — `algorithcoguard`

> Org: [`algorithcoguard`](https://github.com/algorithcoguard). Product: **algorithco guard**, CLI `algo`.
> Status (2026-09-20, verified via `gh repo list algorithcoguard`): only `algorithcoguard/.github`
> (org profile + community health) exists. All repos below are **PLANNED — none created yet**.
> Do NOT create repos without human confirmation. `gh` commands below are documented, NOT executed.

## 1. Repo list (planned)

| Repo | Visibility* | Language | Purpose |
|---|---|---|---|
| `proto` | public† | Protobuf + buf | Contracts: events, decisions, hook↔daemon, daemon↔backend, dataset schema. Tags generate Rust+TS. |
| `core` | public† | Rust (`algo-*`) | Shared libs: types, shell-analysis, policy, redact, provider, fingerprint. |
| `agent` | public† | Rust | Hook client, daemon, adapters, CLI (`algo`), TUI, verifier, loop, scanner, MCP. Signed releases + SBOM. |
| `backend` | private → public later† | Rust | Auth, orgs, signed policy sync, opt-in audit ingest, GitHub App. |
| `dashboard` | private → public later† | TS React+Vite | Team app: history, findings, policy editor, stats. Static SPA. |
| `web` | public | TS Astro+Starlight | Marketing + docs + install script hosting. |
| `eval` | private (datasets may stay private)† | Python | Datasets, harness, thresholds, distillation. Most important in Phase 0. |
| `docs` | public | Markdown | ADRs, threat/security model, roadmap, tracking issues, this plan. |
| `infra` | placeholder (later) | — | Out of scope for now. No repo until its own plan. |

\* Visibility is a proposal — license ADR (D-B/D-07) + legal sign-off decide. Default bias:
permissive for `core`/`agent` (trust for a security tool), restrictive where datasets/user data involved.
† Final call needs the license/name ADRs + human sign-off before public release.

Each repo gets its own `AGENTS.md` (build/test commands, conventions, human-review paths) at creation.

## 2. Branch protection (enforce at repo creation)

- Default branch `main`. Require PR + passing CI (fmt/lint/type, tests, relevant gates per §4) before merge.
- **CODEOWNERS required reviewers** (agent approval never sufficient) on:
  - hard deny list + auto-approve threshold logic
  - redaction rules
  - signature verification (policy bundles, releases, self-update)
  - `algo init` / `algo uninstall` file-modification paths
  - auth (device flow, tokens, orgs/roles)
- `proto`: require `buf lint` + `buf breaking` green on every PR.
- `core`/`agent`: require latency benchmarks + eval false-allow gate green.
- ADR rule: CI blocks any `D-*` implementation PR without a merged ADR in `docs/adr/`.

## 3. Release flow (proto-tag-first, consumers-pin-exact)

1. Contract change → PR to `proto` (with `buf breaking` pass) → merge → **tag** (semver).
2. Tag publishes generated Rust + TS packages.
3. Consumers (`core` → `agent`/`backend` → `dashboard`, `eval` schemas) update in separate PRs,
   each pinning the **exact** `proto` version, generating at build time.
4. Cross-repo sequence tracked by a `docs` issue: `docs` issue → `proto` PR → version bump → consumer PRs.
5. Never commit hand-edited generated code; never duplicate proto-owned types by hand.

`core` distribution: git tags default until the registry ADR (D-D) says otherwise.

## 4. `gh` commands (DOCUMENTED ONLY — do not run without confirmation)

```powershell
# 0. Confirm auth (read-only, safe)
gh auth status

# 1. Re-check what already exists (read-only, safe)
gh repo list algorithcoguard --limit 50

# 2. Create repos (DESTRUCTIVE-ADJACENT — needs human OK per repo; NOT executed in Phase 0 scaffold)
# gh repo create algorithcoguard/proto     --public  --description "algorithco guard: shared protobuf contracts (algorithco_guard.v0)"
# gh repo create algorithcoguard/core      --public  --description "algorithco guard: shared Rust libs (algo-*)"
# gh repo create algorithcoguard/agent     --public  --description "algorithco guard: daemon, hook client, CLI algo, adapters"
# gh repo create algorithcoguard/backend   --private --description "algorithco guard: team cloud API (private until license ADR)"
# gh repo create algorithcoguard/dashboard --private --description "algorithco guard: team dashboard (private until license ADR)"
# gh repo create algorithcoguard/web       --public  --description "algorithco guard: marketing + docs site"
# gh repo create algorithcoguard/eval      --private --description "algorithco guard: datasets + eval harness (datasets stay private)"
# gh repo create algorithcoguard/docs      --public  --description "algorithco guard: ADRs, threat model, roadmap"

# 3. After creation: set defaults, branch protection, CODEOWNERS, required checks per §2
#    (one PR per repo adding AGENTS.md + CODEOWNERS + CI skeleton — Phase 0 P0-DOCS-1/P0-PROTO-1 scope)
```

Why not created now: Phase 0 gate needs license ADR (D-B), name ADR is DECIDED but repo
visibility still needs legal sign-off, and the human must confirm each repo. Scaffold lives
locally until then (see §5).

## 5. Local scaffold dirs (this workspace — Phase 0 stubs)

Created empty with `.gitkeep` + `README.md` stub each. **No product code in Phase 0.**

- `core/` — will hold `algo-*` crates in Phase 1 (types → fingerprint → shell → policy → redact → provider). Now: stub.
- `agent/` — will hold hook client, daemon, Claude-shell adapter, CLI/audit + shadow in Phase 1. Now: stub.
- `backend/` — Phase 3 only. Now: stub.
- `dashboard/` — Phase 3 only (Variant 1 tokens via `design-tokens.css`). Now: stub.
- `web/` — Phase 2 (P2-08) landing + docs (Starlight per `plans/design-tokens.md` §4). Now: stub.

Verify: `Get-ChildItem core,agent,backend,dashboard,web` shows only `.gitkeep` + `README.md` per dir.

## 6. Tracking

One `docs` issue per workstream linking `proto` → `eval` PRs (see `plans/phase-0-00-overview.md` owners).
`infra` out of scope. Human sign-off required: threat model, ToS verdict, gate thresholds
(`docs/exit-gate-P0.md` §5).
