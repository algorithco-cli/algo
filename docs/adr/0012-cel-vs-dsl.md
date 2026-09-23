# ADR-0012: Policy language — CEL vs custom DSL (D-05)

## Status

draft (spike complete 2026-09-23; human merge + sign-off required before
`algo-policy` hardens — `plans/phase-1-01-adr-policy-language.md:3,19`)

## Context

D-05 (`docs/decision-log.md:14`) defers the CEL-vs-DSL decision to P1 and
requires a two-arm spike in `core/crates/policy-spike/` plus a merged ADR
before the `policy` crate hardens. The product engine is currently regex
hard-deny (`core/crates/policy/src/engine.rs:37-49`) and MUST stay so until
this ADR is merged — no product code changes in this spike. Plan bias: CEL
if maturity OK, else DSL (`phase-1-01:19`).

Spike: `cel` crate 0.14.5 vs minimal pest 2.9.2 `deny if ...` DSL, 500
compiled rules (13 real + 487 synthetic) over synthetic shell facts.
Method + full table: `core/crates/policy-spike/README.md`.

## Decision (PROPOSED — pending human merge)

Adopt **CEL** (`cel` crate, pin `>=0.11.4`) as the policy expression
language, subject to the conditions in Verification. The pest DSL grammar
(`core/crates/policy-spike/src/dsl.pest`) is retained as a documented
fallback if CEL conditions fail at merge review.

## Alternatives

- **Custom pest DSL (rejected as primary, kept as fallback)**: 25× faster
  eval worst-case (5.8µs vs 145µs/500 rules), 14× faster load (1.7ms vs
  23ms), dependency-light (`memchr` + `ucd-trie` only). Rejected as primary
  because: no `matches()` regex operator (needed for exact-parity ports of
  `DENY_DD_DEV`, `DENY_EVAL_BASE64`, etc. — would require designing,
  versioning, and fuzzing a new operator ourselves); no external spec,
  editor tooling, or audit ecosystem; every grammar change needs migration
  code. Both clear latency budgets with wide headroom, so speed does not
  decide.
- **Status quo regex-in-Rust (rejected for hardening)**: zero new deps and
  current behavior, but rules are code (each change = release + review of
  `deny_list.rs`), no bundle story for P3 signed policy, no reference
  audit API. Kept ONLY until this ADR merges.

## Consequences

### Positive

- Rules become data: versioned, signed P3 bundles without code changes;
  `Program::references()` gives machine-auditable var/fn lists per rule
  (proven by spike test `cel_references_auditable`).
- Standard spec (google/cel-spec) + C-like syntax + editor support; exact
  regex parity via `matches()` (proven supported by spike probe test).
- Upstream fuzz infrastructure exists; sandbox is non-Turing-complete by
  design (no I/O, no loops).

### Negative

- Dependency weight: `cel` 0.14.5 pulls `antlr4rust`, `chrono`, `uuid`,
  `nom`, `parking_lot` (`cargo tree`, 2026-09-23) — supply-chain surface
  vs 2-crate DSL. `cargo deny/audit` must gate every bump (CI).
- MSRV: `cel` 0.12+ declares MSRV 1.82; workspace is 1.75
  (`core/Cargo.toml:9`). Merging this ADR ratifies MSRV ≥1.82.
- CVE history: CVE-2025-62162 (parser panic, 0.10.0–0.11.3, fixed 0.11.4,
  GHSA-wxwx-9fh7-5mrw) — pin floor `>=0.11.4` is load-bearing, plus the
  spike's never-panics regression test must be ported to product gates.

## Verification

- Spike green: `cargo test -p algo-policy-spike` 11 passed (13/13 deny +
  8/8 abstain parity both arms; `proves_ask_on_*` fail-safe; malformed-input
  never-panics incl. CVE class; `matches()` probe true; `references()`
  auditability) — `core/crates/policy-spike/README.md`.
- Latency (`cargo bench -p algo-policy-spike`, Windows dev 2026-09-23):
  compile-once CEL 23.3ms / DSL 1.7ms; worst-case single decision CEL
  ~145µs / DSL ~5.8µs — both far under L0/L1 p50<3ms/p99<10ms.
  `cargo clippy -- -D warnings` + `cargo fmt --check` clean; full core
  `cargo test --workspace` green (no product regression).
- [VERIFY] maturity links (2026-09-23): repo
  https://github.com/cel-rust/cel-rust (MIT, ~670 stars, active through
  `cel` 0.14.x); changelog+fuzz notes
  https://docs.rs/crate/cel-interpreter/latest/source/CHANGELOG.md;
  API `https://docs.rs/cel-interpreter/latest/cel_interpreter/struct.Program.html`
  (`compile`/`execute`/`references` — additionally proven by compiling
  tests, not docs alone); crate index https://github.com/cel-rust/cel-rust
  (the `cel` facade superseded `cel-interpreter`; crates.io page for the old
  name 404s and the registry page blocks non-browser checks, so no
  crates.io link is recorded); advisory CVE-2025-62162 fixed ≥0.11.4.
- Merge conditions (human): (1) sign below; (2) MSRV ≥1.82 ratified;
  (3) `cel >=0.11.4` floor + never-panics test ported to product gates;
  (4) migration note filed as tracking issue (port 13 rules to CEL bundle,
  keep regex engine until bundle ships). Until then `algo-policy` stays
  regex and this ADR stays draft.

## Sign-off (leave blank — human act)

- [ ] D-05 decision merged: __________ Date: __________
