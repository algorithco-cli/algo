# ADR-0008: Phase 1 scope for Jev — advisory / shadow-only; L3 p50 target stays 250 ms

## Status

draft — 2026-09-20 (owner @algorithcoguard/eval + @algorithcoguard/core)
Preserves `docs/exit-gate-P0.md:1.3` L3 budget; needs human sign-off before any Phase 1 Jev enforcement work.

## Context

P0 Jev multi-region measurement 2026-09-20 on provisional `questions-v0.1-provisional`
(`jev-1.13.0`, 240 records, 3 runs × 2 vantage labels):
`eval/reports/jev-v0.1-vantage-eu-central-2026-09-20.json` —
false_allow 0.000, false_ask 1.000, p50 399ms p99 610ms;
`vantage-us-east` — 0.000 / 1.000, p50 465ms p99 648ms
(plan §P0-JEV-4, `docs/exit-gate-P0.md:1.3`).

Safety (false_allow 0) is not utility: always-ask also scores 0, but shows
`false_ask 1.0` / `allow_rate 0`. Current single-question phrasing shows no
auto-approve value (see §3 gating note — always-ask must not pass).

Latency `p50` is over the `250ms` budget on both vantages (p99 passes).
Calibration is poor: ECE 0.56 / Brier 0.52 (both vantages) on provisional questions.

> **Precondition note 2026-09-23:** R2 (retention SLA) is resolved per owner decision
> (`docs/DEFERRED.md:4.2`, `docs/verify/blocked-on-typesafe.md:R2` — vendor link TBD).
> This changes nothing below: real (non-shadow) Jev traffic stays gated on the redact
> crate + `--show-egress` + `local-only` default + consent flow
> (`docs/redact-consent-readiness.md`) **and** on R1/R3/R4, all still open.

## Decision

Phase 1 Jev stays **advisory / shadow-only**: daemon computes the Jev decision,
writes audit (`shadow: true`, `would_have` + confidence/source/latency), but the
adapter **always renders `approve`** and never blocks on Jev. No Phase 1
enforcement may depend on Jev auto-allow/deny.

The **L3 p50 target stays `< 250 ms`** and the **p99 stays `< 800 ms`**
(`docs/exit-gate-P0.md:1.3` — canonical threshold source; numbers repeated here
so this Decision block stays self-contained). No increase to 500 ms in this ADR.
Revisit only after question tuning (EVAL-6) shows real utility
(before/after: false_ask ceiling, allow_rate, false_allow at fixed ask, AUROC,
ECE/Brier, p50/p99, cost). Latency improvement must come from design
(cache hit-rate, batched questions, prompt brevity, model choice), not silent
budget inflation.

## Alternatives

- **Raise L3 p50 to ~500 ms now**: rejected. Bakes the inefficiency of a
  provisional single-question prompt into the budget and removes the incentive
  to earn throughput via tuning. Reconsidered only with a measured proposal that
  shows tuned questions beat budgets at acceptable utility and cost.
- **Ship Jev enforcement in Phase 1 despite provisional results**: rejected.
  With `false_ask 1.0` there is no user-visible win to justify the extra
  blocking latency and miscalibration risk; fail-safe shadow is the correct
  default until tuning proves otherwise.
- **Drop Jev from Phase 1 entirely**: not chosen here. Shadow keeps the signal
  observable for tuning without user impact; revisited if tuning fails to clear
  the utility + latency + calibration gates after a bounded effort.

## Consequences

### Positive

- Preserves the performance gate as a forcing function for tuning.
- Phase 1 remains useful (shadow + audit + would-have stats) without blocking latency.
- Clear revisit criteria (see Verification) prevents indefinite advisory drift.

### Negative

- No Jev auto-approve benefit in Phase 1 preview (users see `ask` where Jev would ask; no time saved yet).
- Extra shadow traffic and audit volume until tuning lands.
- Requires disciplined shadow review (would-have-blocked) to avoid missing regressions.

## Verification

- L2/distillation is separately and permanently closed per MCA §2.3(b) (see
  `docs/DEFERRED.md:4.3`) — do not confuse Jev's shadow-only status below (which a
  superseding ADR may change) with L2's status (which will not change).
- Phase 1 ships with `shadow: true` only; any PR touching Jev enforcement beyond
  shadow is CI-blocked until this ADR is superseded.
- `docs/exit-gate-P0.md:1.3` still shows `p50 < 250` / `p99 < 800` with measured
  `399/465` and `610/648`; the FAIL is intentionally not papered over.
- Exit for this ADR (superseding ADR): a dated `eval/reports/jev-v0.1-*`
  run on **pinned `questions-v0.1`** (not provisional) that simultaneously shows
  (a) false_allow ≤ thresholds (§1.1) with statistical backing per §3,
  (b) false_ask ≤ ceiling or allow_rate / AUROC headroom versus always-ask
  baseline (see §3), (c) ECE/Brier monotonic post-calibration, and
  (d) p50/p99 + cost/1k within maintained budgets.
  (§1.1/§1.3/§1.5 numbers are canonical in `docs/exit-gate-P0.md`; claim ledger
  in `docs/verify/jev-claims.md`.)

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/eval` + `@algorithcoguard/security` +
  `@algorithcoguard/core` — required before any Phase 1 Jev enforcement or any
  L3 budget change. Without it, Jev stays shadow-only and L3 stays 250/800.
