# Fuzzing — `algorithco guard` (P1-09)

`cargo-fuzz` targets for shell-analysis, policy, and adapter parse.

| Target | Crate entry | Invariant |
|--------|-------------|-----------|
| `fuzz_shell_parse` | `algo-shell-analysis::parse::parse(&str)` | never panic; `Err(ParseError)` maps to **ask**, never `allow` |
| `fuzz_policy_evaluate` | `algo_policy::Engine::evaluate(&str, Profile)` | never panic; returns `Deny` or `Abstain` only, **never `Allow`** on arbitrary input (abstain → ask, fail-safe) |
| `fuzz_adapter_parse` | `algo_adapter_claude::parse::parse_hook(&str)` | never panic; `Err` maps to **ask** via `proves_ask_on_parse_fail` (never `allow`) |

All three targets follow the fail-safe `proves_ask_on_*` contract (`AGENTS.md:6`):
arbitrary `&[u8]` → `if let Ok(s) = std::str::from_utf8(data) { let _ = target_fn(s); }` inside
`libfuzzer_sys::fuzz_target!(|data: &[u8]| { … })` with `#![no_main]`.

## Layout

`fuzz/` is a standalone `cargo-fuzz` crate (not a member of `core`/`agent` workspaces) to avoid duplication:

```
fuzz/
  Cargo.toml              # [package.metadata] cargo-fuzz = true, [[bin]] for each target
  fuzz_targets/
    fuzz_shell_parse.rs
    fuzz_policy_evaluate.rs
    fuzz_adapter_parse.rs
  corpus/                 # optional, checked-in seeds (gitignored)
  artifacts/              # libFuzzer crash artifacts (gitignored)
```

Dependencies use relative paths from `fuzz/`:

- `algo-shell-analysis = { path = "../core/crates/shell-analysis" }`
- `algo-policy         = { path = "../core/crates/policy" }`
- `algo-adapter-claude = { path = "../agent/crates/adapter-claude" }`
- `libfuzzer-sys = "0.4"`

## Prerequisites

```powershell
cargo install cargo-fuzz
rustup toolchain install nightly
```

`cargo fuzz` requires nightly with sanitizer support; `cargo check` on this crate works on stable without `cargo-fuzz`.

## Smoke (PR, 60s each) — required merge gate

```powershell
# --fuzz-dir required (verified 2026-09-23, cargo-fuzz 0.13.2): fuzz/ is a
# STANDALONE crate, but cargo-fuzz discovers <cwd>/fuzz by default, so bare
# `cargo fuzz` fails with "could not find a cargo project" both from root
# and from fuzz/. Always run from the REPO ROOT with --fuzz-dir fuzz.
cargo fuzz run --fuzz-dir fuzz fuzz_shell_parse       -- -max_total_time=60
cargo fuzz run --fuzz-dir fuzz fuzz_policy_evaluate   -- -max_total_time=60
cargo fuzz run --fuzz-dir fuzz fuzz_adapter_parse     -- -max_total_time=60

# or all three via loop
foreach ($t in "fuzz_shell_parse","fuzz_policy_evaluate","fuzz_adapter_parse") {
  cargo fuzz run --fuzz-dir fuzz $t -- -max_total_time=60
}
```

CI PR job `fuzz-smoke` runs the 60s smoke and fails on panic/crash.

## Nightly (1h each) — scheduled

```powershell
cargo +nightly fuzz run --fuzz-dir fuzz fuzz_shell_parse       -- -max_total_time=3600
cargo +nightly fuzz run --fuzz-dir fuzz fuzz_policy_evaluate   -- -max_total_time=3600
cargo +nightly fuzz run --fuzz-dir fuzz fuzz_adapter_parse     -- -max_total_time=3600
```

Recommended CI schedule: nightly `fuzz-nightly` workflow (1h per target, corpus persisted via `actions/cache` on `fuzz/corpus/<target>/`).

## Local quick check without fuzzing

If `cargo-fuzz` is not installed, verify the manifest still type-checks:

```powershell
cargo check --manifest-path fuzz/Cargo.toml
# builds each [[bin]] target (no instrumentation) — should be clean
cargo check --manifest-path fuzz/Cargo.toml --tests
```

## Adding a new target

1. Add `[[bin]] name = "fuzz_new" path = "fuzz_targets/fuzz_new.rs"` to `fuzz/Cargo.toml`.
2. Add dependency if needed.
3. Add `fuzz_targets/fuzz_new.rs` with `#![no_main]` + `fuzz_target!(|data: &[u8]| { if let Ok(s) = std::str::from_utf8(data) { let _ = my_fn(s); } })` and assert no panic / ask on error.
4. Update this README and CI matrix.

## Corpus

- Seed with `fuzz/corpus/<target>/` files (e.g. `ls -la`, `rm -rf /`, `curl … | sh`, JSON hook payloads from `agent/crates/adapter-claude` golden tests).
- Corpus is gitignored except explicit checked-in seeds; libFuzzer persists new coverage-increasing inputs automatically.

## Troubleshooting

- `error: no such command: fuzz` → `cargo install cargo-fuzz`.
- `undefined reference to main` → ensure `#![no_main]` is present and `[package.metadata] cargo-fuzz = true` is set.
- Windows `clang` missing for libFuzzer → install LLVM or use WSL for full `cargo fuzz run`; `cargo check` still passes without clang.
