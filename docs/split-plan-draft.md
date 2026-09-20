# Draft — Repo split plan: monorepo → separate repos (DO NOT EXECUTE)

> **Status:** draft — 2026-09-20 (owner: @algorithcoguard/maintainers)
> **Do not execute until owner approves ADR-0009 (naming) + this plan + license/legal review.**
> This is a **Draft** for review only; no `git filter-repo` or `gh repo create` is run by this file.

## 1. Target state (per owner feedback 2026-09-20)

**Org:** `algorithco` (today: `algorithcoguard` — rename or new org; see §6 Risks).
**Repos (8):**

| Repo | Purpose | Language | Visibility (proposed, pending license) |
|---|---|---|---|
| `guard-proto` | Contracts: events, decisions, hook↔daemon, daemon↔backend, dataset schema. Tags generate Rust+TS. | Protobuf + buf | public (after `docs/adr/0001-license.md` legal sign-off) |
| `guard-core` | Shared libs: types, shell-analysis, policy, redact, provider, fingerprint | Rust (`algorithco-guard-*` crates) | public (same) |
| `guard-agent` | Hook client, daemon, adapters, CLI `algo guard` (`algo-guard` binary), TUI, verifier, loop, scanner, MCP | Rust | public |
| `guard-backend` | Cloud API: auth, orgs, policy distribution, audit ingestion, stats, GitHub App, optional Jev proxy (blocked) | Rust | private → public later |
| `guard-dashboard` | Team web app: history, findings, policies, stats | TypeScript (React) | private → public later |
| `guard-web` | Marketing site + docs + install script hosting | TypeScript (Astro) | public |
| `guard-eval` | Datasets, harness, threshold tuning, human-label training, model export | Python | private (datasets stay private; harness may be public later) |
| `guard-docs` | ADRs, threat model, security model, privacy/dataflow, roadmap, `docs/STATUS-*.md`, `docs/github-org-plan.md` | Markdown | public |

**Monorepo `algorithcoguard/algorithco-guard` becomes:** either archived (tag `monorepo-final`) or kept as a **read-only mirror** with a `MOVED.md` pointer to the 8 repos.

## 2. History preservation — `git filter-repo` (not `filter-branch`)

**Tool:** `git filter-repo` (faster, safer than `filter-branch` or `subtree` alone). Requires a one-time install (`pip install git-filter-repo`).

**Steps (dry-run, no push until sign-off):**

```bash
# 1. Clone monorepo as a bare mirror (once per repo, or use `git filter-repo --path` batch)
git clone --bare https://github.com/algorithcoguard/algorithco-guard.git guard-proto.git
cd guard-proto.git
git filter-repo --path proto/ --path .github/workflows/buf.yml --path README.md --path AGENTS.md --force
# 2. Rewrite remote to new repo (after `gh repo create algorithco/guard-proto --public`)
git remote add origin https://github.com/algorithco/guard-proto.git
git push --mirror origin  # includes tags v0.0.1-alpha (proto) + eval-data-v0.1 (if proto history)

# Repeat for each repo:
# guard-core: --path core/ --path Cargo.toml (if any) --path proto/ (as read-only reference, not copied)
# guard-agent: --path agent/ --path design-tokens.css
# guard-eval: --path eval/ --path .github/workflows/eval.yml
# guard-docs: --path docs/ --path plans/ --path jev-guard-project-plan.md --path AGENTS.md --path README.md
# guard-backend / guard-dashboard / guard-web: --path backend/ etc.
```

**Tag handling:**
- `v0.0.1-alpha` (proto) → lives in `guard-proto` (retain full history); other repos reference it via **pinned tag/SHA** (see §4 Cross-repo versioning).
- `eval-data-v0.1` → lives in `guard-eval`.
- Create a **monorepo-final tag** before split: `git tag monorepo-final 642b4e6 && git push origin monorepo-final` (verified HEAD 642b4e6).

**Why `filter-repo` over `subtree split`:**
- `subtree` keeps full history but not path-renames cleanly for CI; `filter-repo` rewrites paths to repo root (e.g., `proto/algorithco_guard/v0/*` → `algorithco_guard/v0/*` if desired, or keep `proto/` prefix — decide per repo; proposal: keep original path for `guard-proto`, strip prefix for `guard-core` crates).

## 3. Per-repo CI

Each repo gets its own `.github/workflows/`:

| Repo | Workflows |
|---|---|
| `guard-proto` | `buf.yml` (lint + archive-based breaking per `proto/buf.yaml`), CodeQL |
| `guard-core` | `rust.yml` (fmt/clippy/test/deny/audit + fuzz/proptest/mutants + latency bench) |
| `guard-agent` | `rust.yml` + `e2e-headless.yml` (daemon-down/timeout/corrupt-model) + `secrets.yml` (gitleaks/trufflehog) + `links.yml` (docs links) |
| `guard-backend` | `rust.yml` + `migrations.yml` + `secrets.yml` |
| `guard-dashboard` | `node.yml` (Biome/lint/test) + Playwright |
| `guard-web` | `astro.yml` + link check |
| `guard-eval` | `eval.yml` (Python 3.11/3.12 matrix, pip install, ruff/mypy/pytest, harness baselines, artifact upload, gate plumbing) — **currently broken on monorepo due to flat-layout `pip install -e .[dev]`** (see §2 fix already landed `eval/pyproject.toml:tool.setuptools.packages.find`); per-repo eval will be clean |
| `guard-docs` | `links.yml` (fixed globstar loop) + `secrets.yml` (gitleaks OSS docker) |

**Shared CI:** reusable workflows in `algorithco/.github` (org) for `buf` setup, `cargo-deny`, `trufflehog` to avoid 8 copies.

## 4. Cross-repo versioning

**Contracts-first preserved:** `proto` is the contract (`AGENTS.md:4`).

- **Proto → core/agent/backend/dashboard/eval:** `guard-proto` tags `guard-proto-v0.1.0` (or `v0.1.0`) — consumers pin **exact tag/SHA** in `Cargo.toml` (`git = "https://github.com/algorithco/guard-proto", tag = "v0.1.0"`) and `buf.yaml` (`deps: - {name: buf.build/algorithco/guard-proto, version: v0.1.0}`), generate at build (never commit generated).
- **Core → agent/backend:** `guard-core` tags `guard-core-v0.1.0`; `guard-agent` `Cargo.toml` pins `algorithco-guard-types = { git = "...", tag = "guard-core-v0.1.0" }`.
- **No cycles:** DAG `guard-proto → guard-core → guard-agent/backend → guard-dashboard` (per `plans/00-index-build-order.md:54-65`); `guard-eval` tags `guard-eval-v0.1` for datasets.
- **Version bump PRs:** one PR per consumer per proto tag bump, linked to `guard-docs` tracking issue (`docs/tracking-issues.md`).

## 5. Effort estimate

| Work | Effort | Who |
|---|---|---|
| `git filter-repo` dry-runs (8 repos) + verification (`buf lint`, `cargo test` stubs, `pytest` for eval) | 0.5 day | Maintainers |
| Fresh org `algorithco` creation + `gh repo create` 8 repos + branch protection (CODEOWNERS) + `MOVED.md` | 0.5 day | Maintainers + Legal (org name) |
| Per-repo CI wiring (copy + adjust workflows, `eval` pyproject fix already done, `links` fix already done, `secrets` OSS docker) | 1 day | Agent |
| Cross-repo pin migration (update `Cargo.toml`, `buf.yaml`, `README.md`, `AGENTS.md`, `docs/github-org-plan.md`) | 1 day | Core + Agent |
| Docs sweep for new names (ADR-0009 file list — ~40 files) | 1–2 days (if ADR-0009 approved in parallel) | Docs |
| **Total** | **3–4 days** (plus **legal wait** for license/org name) | — |

## 6. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| **Org rename `algorithcoguard` → `algorithco` breaks clones** | All remotes, badges, `cargo` git sources, `gh` scripts | Keep `algorithcoguard` as **redirect** (GitHub org rename preserves redirect) or keep both orgs with mirror + `MOVED.md`; do not delete old org for 12 months. |
| **History rewrite loses contributors/line blame** | Audit trail for `threat-model`/`deny-list` CODEOWNERS review | Use `filter-repo --preserve-commit-hashes` style (preserve original SHAs in commit notes); also push `monorepo-final` tag for full-history reference. |
| **Tag namespace collision** (monorepo `v0.0.1-alpha` vs per-repo `guard-proto-v0.1.0`) | Consumer pin confusion | Adopt **prefixed tags** (`guard-proto-v*`, `guard-core-v*`) from day 1; document in `guard-proto/VERSIONING.md`. |
| **Per-repo CI drift** (8 copies of workflows) | Gate bypass if one repo misses `buf breaking` or `secrets` | Reusable workflows in `algorithco/.github` + required status checks per repo. |
| **Private datasets accidentally pushed public** (eval `seed.jsonl` + `HUMAN-REVIEW.md` contain adversarial data) | Confidential deny-list details + attacker payloads become public | Keep `guard-eval` **private** until license/legal review clears the public/private split per §5 (`docs/public-exposure-review.md`). |
| **Current public monorepo already exposed confidential docs** | `threat-model-v0`, `deny-list` hints, verbatim MCA excerpts (§14.1) are public on `main` | See §5 — move or redact before split; if needed, pub cleanup via `filter-repo` history rewrite **before** pushing to new public repos (BFG). |
| **CI `eval` flat-layout failure repeats** in `guard-eval` | `pip install -e .[dev]` fails as seen at 35518422944 | Already fixed via `tool.setuptools.packages.find` include `["harness*", "baselines*", "tests*"]` — verify in `guard-eval` CI (green locally `pytest 13 passed`). |

## 7. Execution checklist (do not run until sign-off)

- [ ] ADR-0009 approved (naming) + license/legal review on org name `algorithco`.
- [ ] `docs/public-exposure-review.md` (§5) approved — what stays private.
- [ ] `git filter-repo` dry-runs pass (`buf lint`, `pytest` green, no `HIGH_ENTROPY` false positives on paths).
- [ ] `gh repo create algorithco/guard-*` (8) with visibility per §1 table + branch protection (CODEOWNERS) + `MOVED.md` in old monorepo.
- [ ] Push mirrors + tags (`v0.0.1-alpha`, `eval-data-v0.1`, `monorepo-final`) + verify consumer pins.
- [ ] Archive or mirror `algorithcoguard/algorithco-guard` (public) — do not delete for 12 months.

## Sign-off (leave blank — human act)

- [ ] Split plan reviewed: __________ Date: __________
- [ ] Org `algorithco` + repo `guard-*` names approved (ADR-0009): __________ Date: __________
- [ ] Public/private split (`docs/public-exposure-review.md`) approved: __________ Date: __________
- [ ] `git filter-repo` dry-run verified: __________ Date: __________
