# core (Phase 0 — no product code yet)

> Part of **algorithco guard** (CLI algo). Org repo plan: docs/github-org-plan.md.
> Real code lives in algorithcoguard/core starting its build phase. This dir is a scaffold stub.

Scope: shared Rust libraries (algo-* crates): types, shell-analysis, policy, redact,
provider, fingerprint. Heavy fuzz + property + mutation gates. Human review on deny
list, thresholds, redaction, sig-verify.

## Phase 1+ brief

See plans/00-index-build-order.md for build order. Contracts-first: proto tag, pin exact,
generate at build. Fail-safe (ask, never allow), latency budgets in CI, no secrets.

## Now (Phase 0)

Empty except .gitkeep + this stub. Do NOT add product code until docs/exit-gate-P0.md is signed.
