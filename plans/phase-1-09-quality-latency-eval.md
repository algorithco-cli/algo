# P1-09 Quality, latency, eval enforcement (`P1-QUAL`)

## Gates (CI merge-blockers, no warnings)

- `cargo fmt --check`, `clippy -- -D warnings`, `cargo-deny/audit/vet` clean.
- Unit: every crate `cargo test` (parse, normalize, deny, redact, provider mock, adapter golden).
- Property: proptest fingerprint idempotence/key-stability, redact completeness/idempotence.
- Fuzz: `cargo-fuzz` shell-analysis::parse, policy::evaluate, adapter-claude::parse — nightly 1h, PR smoke 60s.
- Mutation: `cargo-mutants --file deny_list.rs,engine.rs` ≥90% killed; survivors → new regression cases.
- Regression corpus: `safe/dangerous/obfuscated.json` incl. `curl|sh`, `base64+eval`, `${IFS}`, unicode homoglyph bins — new bypass becomes permanent case.
- Benchmarks: criterion (`policy_eval`, `fingerprint_normalize`, `redact_10k`, `pipeline_L0L1`) + hyperfine hook-client cold start. `latency-budget` job fails on >10% regression or absolute breach (L0/L1 p50<3ms/p99<10ms, L3 p50<250ms/p99<800ms). Reference runner pinned, baseline artifacts uploaded. Adjust only via ADR.
- Eval gate: harness runs Mock+Jev over versioned dataset; publish false-allow/ask/deny/p50/p99/cost; block if false-allow > P0 threshold.
- E2E: headless Claude Code 10 scenarios (safe ls, dangerous rm -rf /, obfuscated pipe) shadow + enforcing dry-run; `algo uninstall` restore check.
- Human review: CODEOWNERS on deny_list, builtin policy, redact/patterns, `algo init`/`algo uninstall`, transport/auth — agents cannot self-merge.

## Pipeline discipline

L0/L1 sync, zero-alloc hot path (SmallVec, Arc<CompiledRules>), no regex in L0 (aho-corasick only in redact off-path), no network in L0/L1. L3 batched + pooled h2 + 700ms timeout counted separately.
