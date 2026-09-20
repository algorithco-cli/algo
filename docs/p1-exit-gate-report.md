# P1 Exit-gate report (private MVP, 2026-09-21)

Per `plans/phase-1-10-exit-gate-runbook.md:3`. Gate stays **deferred** per waiver
`docs/adr/0009-p0-gate-waiver.md` — sign-offs blank, no `__________` filled by agents.
This file records **local evidence** only.

## 1. cargo test + proptest + fuzz-smoke + mutants

- `core`: `cargo test` green — types 5, redact 9, fingerprint 7, shell-analysis 10,
  policy 8 + regression 2 (`safe 20 / dangerous 20 / obfuscated 20`), provider 5.
  Fix 2026-09-21: `DENY_DD_DEV` now matches `nvme0n1`/`sda1` suffixes
  (`core/crates/policy/src/deny_list.rs`), `regression.rs` import
  `PolicyDecision`, `mock.rs` unused import removed.
- `agent`: `cargo test` green — adapter 52, audit 6, cli 5, daemon 19+15 lib,
  hook-client 4, tui 9. Total agent 86+.
- `backend`: `cargo test` 45 passed (verify tampered/rollback/expired/secret/unauth).
- `eval`: `PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 python -m pytest
  tests/test_runner_failsafe.py tests/test_dataset_schema.py tests/test_no_secrets.py`
  **13 passed**. Fix: `regression-corpus` added to `SKIP_DIRS`
  (`eval/tests/test_no_secrets.py`) — synthetic base64 `curl|sh` vectors
  (`Y3VybCBodHRwOi8vYSB8IHNo`) trip generic entropy heuristic, no credentials.
- `cargo fmt --check`: clean for `core`, `agent`, `backend` (ran `cargo fmt`).
- `cargo clippy -- -D warnings`: clean for `core`, `agent`, `backend`.
- `cargo deny check` / `cargo audit`: **not installed locally** (`no such command: deny`);
  config `deny.toml` + `core/deny.toml` present, CI runs gate. No new bans.
- `cargo fuzz`: `cargo check --manifest-path fuzz/Cargo.toml` clean
  (`fuzz_shell_parse`, `fuzz_policy_evaluate`, `fuzz_adapter_parse`, libfuzzer 0.4).
  Full 60s smoke + nightly 1h are CI jobs (`fuzz/README.md`), not run locally.
- `cargo mutants`: config `.cargo-mutants.toml` + `core/.cargo-mutants.toml`
  (`deny_list.rs,engine.rs` ≥90% killed), runner `scripts/mutants.{sh,ps1}`.
  Full run not executed locally (long); `flip_any_deny_mutant_caught` +
  `every_deny_has_rule_id` + regression corpus are local proxies. Survivors →
  `eval/regression-corpus/*.json` per `eval/regression-corpus/README.md`.

## 2. Criterion / latency

- Benches compile: `cargo check --benches` clean for `core` + `agent`
  (`policy_eval`, `fingerprint_normalize`, `redact_10k`, `pipeline_L0L1`).
- Unit latency proofs green: `pipeline::latency_under_3ms_for_l0_l1`,
  redact `149µs/10KB` release (`examples/latency.rs`) <500µs.
- Absolute budgets enforced by `scripts/latency-budget.{sh,ps1}` in CI:
  L0/L1 p50<3ms p99<10ms, L3 p50<250 p99<800 (report-only for mock),
  regression >10% fails. `hyperfine` hook-client cold start ~1ms reference.
- Local `cargo bench` full run not executed (CI `p1-exit` baseline job).

## 3. Eval gate

- P0 dataset `eval/datasets/v0.1/seed.jsonl` 240 synthetic, rules_only v0.2.0
  realistic baseline. Jev provisional `false_allow 0.000 false_ask 1.000`
  shadow-only per ADR-0008 — Jev stays OFF (mock only).
- Regression corpus is the fast local hard-deny pin (no Jev):
  `eval/regression-corpus/{safe,dangerous,obfuscated}.json` 20+20+20,
  bins `curl|sh`, `base64|sh`, `eval+base64`, `${IFS}`, homoglyph+fallback.
  Pure homoglyph without fallback is known bypass → NFKC normalization
  (see README §Bins, `docs/SECURITY-REVIEW-QUEUE.md`).

## 4. Install matrix

- `algo init→doctor→uninstall` byte-identical covered by
  `agent/crates/cli/src/main.rs:820` `init_doctor_uninstall_byte_identical`
  (TempDir `ALGO_HOME`, additive merge, backup `*.algo-backup-<ts>`).
- `algo pause` daemon-killed covered by `pause_bypasses_daemon_killed`.
- macOS 14 arm64 + Ubuntu 22.04/24.04 fresh-VM matrix is CI/manual,
  not run locally (Windows dev machine). Script per runbook §3.

## 5. Shadow soak

- `shadow_counts` test: 500 shadow decisions, `would-have-blocked N`,
  never blocks (adapter always approve in shadow).
- Multi-user soak (≥3 users × ≥3 days, ≥500 decisions, would-have review,
  zero incidents) is **pending human operation** — tracked in `docs/DEFERRED.md`.

## 6. Human sign-off (before P2)

Blank per waiver: deny-list, redact patterns, ADR CEL/DSL (P1-01 spike
`core/crates/policy-spike/` not yet run — current engine is regex hard-deny,
CEL decision deferred), install paths. See `docs/SECURITY-REVIEW-QUEUE.md`
S01–S16 (S07–S10 now Written 2026-09-21, queued).

## Fail-safe re-verification

- Grep `=> Allow|map_err.*Allow|catch.*Allow` in `core/agent/backend`: only
  `backend/src/stats.rs:56` `"allow" => allow += 1` (counter, not decision).
- `proves_ask_on_*` count 32 (20 fns): types, redact, shell parse/facts,
  policy abstain/no-match, provider timeout/parse, adapter parse,
  daemon db/provider/channel, audit not-redacted, auth unauthed, verify error.
- `allowlist + rm -rf /` still deny via `DENY_RM_RF_ROOT` (regression).

## Assumptions

- `cargo-deny/audit`, `cargo-mutants` full, `cargo-fuzz` smoke, `cargo bench`
  full, macOS/Linux matrix, multi-user shadow soak run in CI/human ops, not locally.
- Homoglyph pure bypass accepted as known gap until NFKC lands.
- Jev remains OFF; no real user data sent.
