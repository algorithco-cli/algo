# Decision log — algorithco guard

Source: `../plans/90-crosscutting-gates-ux-decisions.md` section 3 + `../jev-guard-project-plan.md` section 9.
ADR required before code for every open item. Branch `adr/D0X-*`.

## D-01..D-10 (canonical)

| ID | Title | Blocks | Default / Proposal | Owner | Date | Status |
|----|-------|--------|--------------------|-------|------|--------|
| D-01 | Jev BYOK vs proxy | P3 scope/privacy/cost | BYOK only — no proxy mode will be built | @algorithcoguard/backend | 2026-09-20 | accepted (BYOK only — see [ADR 0004](./adr/0004-byok-vs-proxy.md)) |
| D-02 | Jev API + SDK | all L3 | no default until verified (links+dates) | @algorithcoguard/core | 2026-09-20 | verify-open |
| D-03 | ToS training on Jev outputs | P3 L2 | Jev-output distillation not built — permanently out of scope | @algorithcoguard/eval | 2026-09-20 | closed — rejected by MCA §2.3(b), will not build (see `docs/DEFERRED.md:4.3`) |
| D-04 | Hook caps per agent | P2 stop/edit, P4 adapters | per-agent spike docs first | @algorithcoguard/agent | 2026-09-20 | draft |
| D-05 | CEL vs DSL | P1/P2 policy | spike both, bias CEL if mature | @algorithcoguard/core | 2026-09-20 | spike complete 2026-09-23, ADR-0012 draft (CEL proposed, human merge pending — policy stays regex until merged) |
| D-06 | ort vs candle | P3 L2 runtime | benchmark both | @algorithcoguard/agent | 2026-09-20 | deferred to P3 |
| D-07 | License per repo | public release | permissive core/agent proposal, legal sign-off pending | @algorithcoguard/legal | 2026-09-20 | draft (see [ADR 0001](./adr/0001-license.md)) |
| D-08 | Payments MoR | monetization | external MoR, early decide; reserve entitlement field in P3 | @algorithcoguard/backend | 2026-09-20 | deferred (see [ADR 0006](./adr/0006-payments-mor.md)) |
| D-09 | Windows priority | P4 Windows | macOS+Linux first, harden P4 unless pulled forward | @algorithcoguard/agent | 2026-09-20 | draft (see [ADR 0007](./adr/0007-windows-scope.md)) |
| D-10 | Name / branding | packages, web | DECIDED — see [ADR 0002](./adr/0002-naming.md) | @algorithcoguard/maintainers | 2026-09-20 | accepted |

## D-A..D-F (Phase 0 drafts, mapped to canonical)

| Draft | Title | Maps to | Proposal | Owner | Date | Status |
|-------|-------|---------|----------|-------|------|--------|
| D-A | BYOK vs proxy | D-01 | BYOK first, proxy optional later | @algorithcoguard/backend | 2026-09-20 | draft (see [ADR 0004](./adr/0004-byok-vs-proxy.md)) |
| D-B | License | D-07 | permissive core/agent, legal sign-off pending | @algorithcoguard/legal | 2026-09-20 | draft |
| D-C | Name | D-10 | product algorithco guard, CLI algo, proto algorithco_guard.v0, crates algo-*, home ~/.algo/ | @algorithcoguard/maintainers | 2026-09-20 | accepted |
| D-D | Core distribution | — (master plan section 3) | git tags default until private registry exists | @algorithcoguard/core | 2026-09-20 | draft (see [ADR 0005](./adr/0005-core-distribution.md)) |
| D-E | Payments MoR defer | D-08 | external MoR, decide early; no build in P0 | @algorithcoguard/backend | 2026-09-20 | deferred (see [ADR 0006](./adr/0006-payments-mor.md)) |
| D-F | Windows scope | D-09 | macOS+Linux first | @algorithcoguard/agent | 2026-09-20 | draft (see [ADR 0007](./adr/0007-windows-scope.md)) |
| D-G | Jev Phase 1 shadow-only | — (P0 Jev measurement) | advisory/shadow-only; L3 250/800 stays; revisit after question tuning | @algorithcoguard/eval+core | 2026-09-20 | draft (see [ADR 0008](./adr/0008-jev-shadow-only.md)) |
| D-H | P0 gate waiver (ADR-0009) | P0 exit gate | waiver retired, gate enforced as written | @algorithcoguard/maintainers | 2026-09-24 | **retired** (see [ADR 0009](./adr/0009-p0-gate-waiver.md)) |
| D-I | §3b remediation (quarantine vs scoped exception) | P0 exit gate §3b FAIL | scoped exception proposed, quarantine fallback | @algorithcoguard/maintainers | 2026-09-24 | draft (see [ADR 0015](./adr/0015-s3b-remediation.md)) |

## Deferred explicitly

- CEL/DSL detailed design -> P1 (`P1-CORE` policy spike).
- ort/candle benchmark -> P3 (L2 runtime).
- Speculative evaluation -> deferred until pipeline measured.

Each ADR lives in [adr/](./adr/README.md) as `NNNN-title.md` with Status/Context/Decision/Alternatives/Consequences/Verification.
