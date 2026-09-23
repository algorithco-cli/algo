# Phase 3 — L2 Local Model [RETIRED SCOPE — Jev-output distillation CLOSED 2026-09-23]

> **Retired scope (owner decision 2026-09-23, see `docs/DEFERRED.md:4.3`).**
> Training an L2 local model on Jev/System One outputs is **contractually prohibited**
> by TypeSafe MCA §2.3(b) — *"Customer will not do (and will not attempt to do)… any
> of the following: … (b) use the Services or any Output (defined below) to perform
> model distillation, train a model to imitate the output of the Services, or develop
> (or to facilitate the development of) a similar or competing product or service"*
> (verbatim; cf. `docs/verify/jev-tos.md:Q1`). This is a confirmed, permanent
> restriction under the current MCA — not a temporary blocker, not awaiting a
> TypeSafe response. No exception ticket will be filed.
>
> **What remains allowed:** L2, if ever built, may **ONLY** use human-labeled data,
> deterministic rule outputs, or user confirmations as training signal — **never**
> Jev/System One outputs (no `jev_label`/`jev_confidence`/`jev_probabilities` labels
> or features, no Jev-output training log). The human-only training checklist below
> stays as the standing gate for that path. The `[DECISION]` ort-vs-candle runtime
> choice (D-06) is unaffected and stays deferred to Phase 3.
>
> **Historical record:** the prior body of this file (human-only redesign draft with
> P3-07/P3-08 build planning, written while a written exception was still an open
> option) is preserved in git history — this file was converted to a closed decision
> record, not deleted, so the reasoning trail survives.

## Human-only training checklist (standing gate — must be true before any L2 artifact ships)

- [ ] No dataset `label` or `rationale` was produced, refined, or filtered by a Jev answer.
- [ ] No training feature column is a Jev output (`jev_label`, `jev_confidence`, `jev_probabilities`, `jev_score`).
- [ ] No logging of Jev outputs for future L2 (the provider `JevClient` is shadow-only per ADR-0008 and does not write a training log).
- [ ] Every new record logs `annotator: human-*` and `source: human-label | deterministic-rule | user-confirmation`.
- [ ] Every training run records `trained_on` + dataset SHAs + annotator list in the eval report.
- [ ] L2 ships only if it meets the same eval gate as Jev on its share (false_allow + G1 false_ask + G2 vs `rules_only` + ECE/Brier + p50/p99) with statistical backing.

## Caveats (standing)

- No unredacted secrets/code in any training input (redact + `local-only` default, see `docs/redact-consent-readiness.md`).
- Jev-output distillation is not a fallback, a stretch goal, or a "revisit later" item — it is closed.
