# Phase 3 — L2 Local Model (Human-only training, Gated) [DRAFT — no distillation on standard terms]

> **PROHIBITED on standard MCA** to train a model on Jev outputs without a written
> exception — MCA §2.3(b) (see `docs/verify/jev-tos.md:Q1`). Redesign: L2 trains
> **only on human labels, deterministic rule outputs, and user confirmations**,
> never on Jev outputs. Log no Jev-derived label or feature (see §1 checklist).
> Needs TypeSafe written exception to re-enable Jev-distilled training. Status:
> **Draft** — human review required before any L2 training. Ships only if
> false-allow ≤ thresholds (§1) with statistical backing + meets L2 budgets.

## P3-07 Eval pipeline (`eval`) — human-only

- **Gate first:** MCA Sep 19, 2026 quote (see `docs/verify/jev-tos.md:Q1–Q7` +
> measured DPA summary at jev-tos §4.4 `typesafe.ai/data-processing` Apr 24, 2026)
> plus either a **written TypeSafe distillation exception** (if pursuing Jev-distilled)
> **or** the **human-only redesign** (this document). Consent design (opt-in, redacted, no secrets/code).
> Fallback if denied: user-confirmation-only or no-L2.
- **Dataset provenance gate (human-only):** every `dataset_version` added in Phase 3
> carries `source: human-label | deterministic-rule | user-confirmation` provenance;
> **no** `source: jev-output` or `jev_label`/`jev_confidence` feature (see §1 checklist).
> Harness `threshold_sweep` and ONNX export run only on human-origin labels.
- Versioned dataset growth (obfuscated/adversarial per EVAL-6 diversity), harness over
> any `DecisionProvider` **excluding Jev-distilled providers** until exception, metrics
> (false-allow/ask/deny, calibration, p50/p95/p99, cost), threshold tuning per profile,
> ONNX export + versioned artifact.
- AC: eval report published with calibration curve + human-only provenance; CI gate consumes it.

## P3-08 Agent runtime (`agent`) — human-only

Behind `DecisionProvider` trait (swap without pipeline touch). Load versioned ONNX from eval
(**human-only artifact**). [DECISION] `ort` vs `candle`: benchmark both (p50/p99, binary size, musl/Win/macOS, ONNX fidelity, maintenance) in ADR before code.

- Enforce L2 <10ms p50/<25ms p99 in CI; fallback to L3 on missing/slow/corrupt model; musl static still works; binary-size measured.
- AC: parity gate passes; deleted/corrupt model → ask-via-L3 proven; **provenance gate:**
> loaded artifact records `trained_on: human-only (no Jev outputs)` + human reviewer.

## 1. Human-only training checklist (must be true before any L2 artifact ships)

- [ ] No dataset `label` or `rationale` was produced, refined, or filtered by a Jev answer.
- [ ] No training feature column is a Jev output (`jev_label`, `jev_confidence`, `jev_probabilities`, `jev_score`).
- [ ] No logging of Jev outputs for future L2 (the provider `JevClient` is shadow-only per ADR-0008 and does not write a training log).
- [ ] Every new record added in Phase 3 logs `annotator: human-*` (not `agent-synthetic-v0.1` alone) and `source` as above.
- [ ] Every training run records `trained_on` + dataset SHAs + annotator list in the eval report.

Until a written TypeSafe exception is filed and a **distilled** ADR supersedes this draft,
the checklist **blocks** any L2 PR that introduces a Jev-derived training dependency.

## Caveats (from §4.1, must resolve)

- ToS permission remains **PROHIBITED** for distillation without a written exception (`§2.3(b)`); consent is human-only.
- No unredacted secrets/code in any training input (redact + `local-only` default, see `docs/redact-consent-readiness.md`).
- L2 evaluated on same set + same gate as Jev (false_allow + G1 false_ask + G2 vs `rules_only` v0.2.0 + ECE/Brier + p50/p99).
