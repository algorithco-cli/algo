# Roadmap — Phase 0 to Phase 4

Source: `../plans/00-index-build-order.md` and `../jev-guard-project-plan.md`.
Build order is strict: `proto -> core -> agent/backend -> dashboard`. `eval` gates thresholds.

## Phase 0 — Contracts and evidence (now, docs only)

- Docs skeleton, ADR process, threat-model v0 (this folder).
- `proto` v0: canonical events, decision, dataset schema. `buf lint` + `buf breaking` green, tagged.
- `eval`: 200-300 labeled actions, harness, baselines.
- `jev`: API/SDK dossier, ToS check, probe, latency/accuracy measurement.
- Exit gate: measured Jev false-allow + L3 p50 < 250ms / p99 < 800ms. Fail means redesign, no product code.

## Phase 1 — Local MVP (no cloud)

- `core`: types, fingerprint, shell-analysis, policy + hard deny list, redact, provider (Jev + mock).
- `agent`: daemon + hook client, Claude Code shell-only adapter, SQLite audit, `init/doctor/status/why/log/pause`, shadow mode.
- Exit gate: L0/L1 p50 < 3ms / p99 < 10ms, eval gate pass, macOS+Linux init/uninstall byte-identical, 3+ users x 3+ days shadow.

## Phase 2 — Enforcement and second capability

- Enforcement with `eval` thresholds. Verifier (stop hook) + loop controller. Edit/write coverage.
- `web`: landing + docs.
- Exit gate: prompt-reduction measured, false-allow within threshold, no fail-open in review + fuzz.

## Phase 3 — Cloud and teams

- `backend`: auth device flow, orgs, signed policy sync, opt-in audit ingestion.
- `dashboard`: history, stats, policy editor dry-run.
- L2 local model only if ToS-cleared, parity-gated.
- Exit gate: login -> org -> policy -> daemon -> audit -> dashboard E2E, backup/restore tested.

## Phase 4 — Expansion

- Codex + OpenCode adapters (spike first, degrade gracefully).
- Scanner, Choice-only question assistant, TUI, GitHub App, SSO/SCIM, Windows hardening.
- Exit gate: adapter E2E + degradation matrix, scanner precision/recall published, Windows matrix green.
