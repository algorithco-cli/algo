# P1-10 Exit-gate runbook

## Checklist (all must pass, human sign-off)

- [ ] `cargo test + proptest + fuzz-smoke + mutants(deny≥90%)` green on core; clippy/deny/audit clean.
- [ ] criterion: L0/L1 p50<3ms p99<10ms; hook-client ~1ms; L3 p50<250ms reported.
- [ ] Eval gate: false-allow ≤ P0 threshold, report published with dataset version.
- [ ] `algo init`/`algo doctor`/`algo uninstall` byte-identical on macOS 14 arm64 + Ubuntu 22.04/24.04; `algo pause` daemon-dead.
- [ ] Shadow soak: ≥3 users × ≥3 days, ≥500 decisions, would-have stats reviewed, zero enforcement incidents.
- [ ] Human sign-off on deny-list, redact patterns, ADR CEL/DSL, install paths before P2.

## Runbook

1. `cargo bench -- --save-baseline p1-exit` + `hyperfine` hook-client; attach artifacts.
2. `eval` harness vs `eval-data-v0.1`; publish JSON+md.
3. Fresh-VM install matrix script (`algo init→algo doctor→algo uninstall→diff-against-backup`).
4. Shadow users export `audit.db` redacted stats → review would-have-blocked.
5. Gate meeting: ratify P2 thresholds, sign.

## Fail-safe re-verification

Grep: no `=> Allow` in any catch/map_err path. Each new I/O/timeout/parse path has `proves_ask_on_*` test. `allowlist + rm -rf /` still deny.
