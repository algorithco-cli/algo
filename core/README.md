# core (Phase 0 — no product code yet)

> Part of **algorithco guard** (CLI algo). Monorepo: `algorithcoguard/algorithco-guard`.
> Real code lives in the monorepo subdirectory `core/` starting its build phase. This dir is a scaffold stub.

Scope: shared Rust libraries (algo-* crates): types, shell-analysis, policy, redact,
provider, fingerprint. Heavy fuzz + property + mutation gates. Human review on deny
list, thresholds, redaction, sig-verify.

## Phase 1+ brief

Contracts-first: proto tag, pin exact,
generate at build. Fail-safe (ask, never allow), latency budgets in CI, no secrets.

## Now

Product code lives here (shared `algo-*` library crates).
