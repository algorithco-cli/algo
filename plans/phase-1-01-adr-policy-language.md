# P1-01 ADR: Policy Language — CEL vs Custom DSL [DECISION #5]

> Must merge before `policy` crate hardens. Human sign-off required.

## Spike (2 days, `core/crates/policy-spike/`)

- (a) `cel-rust`: 500 rules over parsed facts. Check maturity, sandbox, fuzz-ability.
- (b) Minimal `pest`-based DSL: `deny if cmd in [...] && flag...`.
- Measure: compile-once time, eval p50/p99, binary size, auditability, deny-list expressiveness, versioning story.

## Decision inputs

- cel-rust maturity/perf check with links+dates.
- Custom-DSL cost estimate (grammar, tooling, editor support).
- Compile-once-at-load + rollback story for signed bundles (P3).

## Acceptance

ADR merged with latency table + decision + migration note. `policy` not started until approved. Bias: CEL if maturity OK, else DSL.

## Risk if skipped

High — wrong choice bakes perf/UX debt into every decision.
