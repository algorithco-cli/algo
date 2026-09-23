# policy-spike (P1-01 — SPIKE ONLY, not product code)

Per `plans/phase-1-01-adr-policy-language.md`: 2-day spike, `cel` crate vs
minimal pest DSL over synthetic shell facts. No product crate may depend on
this crate. Decision lands in `docs/adr/0012-cel-vs-dsl.md` (draft — human
merge + sign-off required before `algo-policy` hardens).

## Method

- Facts (`src/facts.rs`): `{bins, flags, toks, has_pipe_to_shell, net, raw}`
  from a naive whitespace tokenizer. Deliberately NOT the product
  `tree-sitter-bash` parser — the spike compares *language* cost, not parsing.
- Corpus (`src/corpus.rs`): 13 real deny rules (mirroring product
  `deny_list.rs` semantics) + 487 synthetic variants = 500 compiled rules.
- CEL (`src/cel_engine.rs`): `Program::compile` once at load; per decision a
  fresh `Context::default()` + 5 vars, deny if any program evals `true`.
- DSL (`src/dsl.rs` + `src/dsl.pest`): `deny if ...` with `in` / `contains` /
  `==` / `!=`, `&&` / `||` / `!`, parens. Parse-once, AST-walk per decision.
- Fail-safe: compile/parse/eval error → `Ask`, never `Allow`; empty rule
  set → `Ask`. Proven by `proves_ask_on_*` tests in both engines.

## Results (Windows dev machine, 2026-09-23, `cargo bench`)

| Measure | CEL (`cel` 0.14.5) | pest DSL (2.9.2) | Budget |
|---|---|---|---|
| Compile/parse-once, 500 rules | ~23.3 ms | ~1.7 ms | one-time at load |
| Single decision, worst case (safe input, scans all 500) | ~145 µs | ~5.8 µs | L0/L1 p50<3ms / p99<10ms |
| Single decision, best case (first-rule hit) | ~32 µs | ~32 ns | — |
| 13/13 real rules deny + 8/8 safe abstain | ✅ | ✅ | parity |
| `matches()` regex macro | ✅ supported (probe test) | ❌ needs new operator | expressiveness |
| Malformed-input panic (CVE-2025-62162 class) | ✅ no panic, Err (test) | ✅ no panic, Err (test) | fail-safe |
| Reference audit (`Program::references()`) | ✅ vars/fns enumerable (test) | grammar file is the surface | auditability |
| Dependency weight (`cargo tree`) | heavy: `antlr4rust`, `chrono`, `uuid`, `nom`, `parking_lot`… | light: `memchr`, `ucd-trie` only | supply-chain |

Both clear the latency budget with wide headroom (CEL ~20×, DSL ~500×).
The decision therefore turns on maturity/auditability/supply-chain, not speed —
see ADR-0012 (bias per plan: CEL if maturity OK).

## CEL maturity notes ([VERIFY] 2026-09-23)

- Repo `github.com/cel-rust/cel-rust`, MIT, ~670 stars, active
  (releases through `cel` 0.14.x; `cel-interpreter` 0.10.0 2025-07-23).
- Fuzz infrastructure exists upstream (Value-binop target + fuzz-test fixes
  in 0.10.0 changelog) — https://github.com/cel-rust/cel-rust,
  https://docs.rs/crate/cel-interpreter/latest/source/CHANGELOG.md.
- CVE-2025-62162: parser panic on malformed input, cel-rust 0.10.0–0.11.3,
  fixed in 0.11.4 (GHSA-wxwx-9fh7-5mrw). Spike pins `cel 0.14` + a
  never-panics regression test. Product adoption MUST keep `>=0.11.4` and the
  no-panic test (gate before P2).
- API used: `cel::{Context, Program}`, `Program::compile(&str)`,
  `Context::add_variable`, `program.execute(&ctx)`,
  `program.references().has_variable(..)` — all verified by compiling tests,
  not by reading docs alone.
- MSRV note: `cel` 0.12+ declares MSRV 1.82; workspace is 1.75
  (`core/Cargo.toml:9`). Adopting CEL as a product dep raises MSRV —
  recorded in ADR-0012 alternatives/consequences.
