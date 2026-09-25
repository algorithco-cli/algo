<p align="center">
  <img src="web/public/logo.svg" alt="algorithco guard logo" width="128" />
</p>

<h1 align="center">algorithco guard</h1>

<p align="center">
  Intelligent control layer for CLI coding agents (Claude Code, Codex CLI, OpenCode).<br />
  Attaches via hooks / plugins / MCP. Makes agents safer, quieter, and auditable.
</p>

<p align="center">
  CLI: <code>algo</code> · Naming: <a href="docs/adr/0002-naming.md">ADR-0002</a>
</p>

## Contents

- [CLI](#cli)
- [Run everything locally](#run-everything-locally-no-docker)
- [Decision pipeline (L0 → L4)](#decision-pipeline-l0--l4)
- [Repo layout](#repo-layout--dependency-dag)
- [Build & test per package](#build--test-per-package)
- [Non-negotiables](#non-negotiables)
- [Docs](#docs)
- [Status](#status)

## CLI

```
algo init | doctor | status | why | log | enforce | pause | login | policy
```

| Command | Purpose |
|---|---|
| `algo init` | Detect agents → show diff → per-agent consent → backup + additive hook install → privacy prompt → `algo doctor`. ~30s target. |
| `algo doctor` | Verify install, daemon, socket, DB, agent configs. |
| `algo status` | Counts (allowed / asked / blocked), savings, profile + threshold source. |
| `algo why` | Explain last decision: action + reason + confidence + source + latency. |
| `algo log` | Local audit log. `--show-egress` inspects exactly what would leave the machine. |
| `algo enforce [on\|off]` | Shadow → enforce switch. Explicit opt-in after shadow trust. |
| `algo pause` / `algo resume` | One-step stop / resume. Works even if daemon is broken. |
| `algo login` | OAuth device flow for team/cloud features (Phase 3). |
| `algo policy` | View / dry-run policy bundles. |

## Run everything locally (no Docker)

Three processes, all verified working:

| Service | How to run | URL |
|---|---|---|
| Backend (`algo-backend`, in-memory, no cloud account needed) | `cargo run --manifest-path backend/Cargo.toml` | `http://127.0.0.1:8080/` (API console) |
| Web (static docs/marketing site) | `cd web && npm run preview` | `http://127.0.0.1:3007/` |
| Dashboard (static team SPA) | `cd dashboard && npm run preview` | `http://localhost:4173/` |

Requires Rust stable, Node ≥ 20, and `npm install` (`npm ci`) inside `web/` and `dashboard/` first.

## Local paths (convention, DECIDED)

- Home: `~/.algo/`
- Socket: `~/.algo/algo.sock` (Windows: named pipe, see Phase 4)
- DB: `~/.algo/audit.db` (SQLite WAL — local audit log, cache, preferences)

## Decision pipeline (L0 → L4)

Most requests never reach Jev. Hypotheses — measured in Phase 0/1, not facts.

| Level | What | Target |
|---|---|---|
| L0 | Hard deny/allow rules on shell syntax tree | p50 < 3 ms / p99 < 10 ms |
| L1 | Cache by normalized action fingerprint | p50 < 3 ms / p99 < 10 ms |
| L2 | Small local classifier (CPU, human-labeled/deterministic training only — never Jev outputs, MCA §2.3(b)) | p50 < 10 ms / p99 < 25 ms |
| L3 | Jev evaluation (remote) | p50 < 250 ms / p99 < 800 ms |
| L4 | Ask the user (universal fallback) | human |

Fail-safe: any error / timeout / crash / parse-fail → `ask`. Never `allow`.

## Repo layout / dependency DAG

Monorepo `algorithcoguard/algorithco-guard` (private until release): `proto → core → agent/backend → dashboard`, `eval` gates thresholds.

## Build & test per package

| Package | Commands |
|---|---|
| `proto/` | `buf lint` · `buf breaking --against .git#branch=main` |
| `core/`, `agent/`, `backend/` | `cargo test` · `cargo clippy -- -D warnings` · `cargo deny check` · `cargo audit` |
| `eval/` | `pytest` · `ruff check .` · `mypy .` |
| `web/`, `dashboard/` | `npm run lint` · `npm run test` · `npm run build` |

Per-crate details live in each package's `README.md`; working agreement in [`AGENTS.md`](AGENTS.md).

## Non-negotiables

1. Fail-safe → `ask`, never `allow` on error/timeout/crash/parse-fail.
2. Deterministic rules outrank models.
3. Latency budgets in CI — over-budget = no merge (or ADR).
4. Local-first, privacy-by-default (redact before egress, `local-only/redacted/full`), explainable (`algo why`), reversible install, provider abstraction, no invented APIs (`[VERIFY]`).

## Docs

- Working agreement: [`AGENTS.md`](AGENTS.md)
- Contributing: [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md)
- Decisions: [`docs/adr/`](docs/adr/README.md) (naming: [`0002-naming`](docs/adr/0002-naming.md))
- Brand tokens: [`design-tokens.css`](design-tokens.css) (Variant 1, single source — no hard-coded hex)

## Status (Phase 0)

- [ ] proto v0 tagged
- [ ] dataset v0.1 tagged
- [ ] harness + baselines green
- [ ] Jev multi-region report published
- [ ] gate signed, Phase 1 unblocked
