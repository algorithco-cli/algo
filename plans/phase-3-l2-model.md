# Phase 3 — L2 Local Model (Distillation, Gated)

> Blocked until ToS [VERIFY] cleared in writing. Ships only if false-allow ≤ Jev threshold on its share + meets L2 budgets.

## P3-07 Eval pipeline (`eval`)

- Gate first: ToS quote + TypeSafe written answer; consent design (opt-in, redacted, no secrets/code). Fallback if denied: user-confirmation-only or no-L2.
- Versioned dataset growth (obfuscated/adversarial), Jev question-phrasing experiments, harness over any DecisionProvider, metrics (false-allow/ask/deny, calibration, p50/p95/p99, cost), threshold tuning per profile, ONNX export + versioned artifact.
- AC: eval report published with calibration curve; CI gate consumes it.

## P3-08 Agent runtime (`agent`)

Behind `DecisionProvider` trait (swap without pipeline touch). Load versioned ONNX from eval. [DECISION] `ort` vs `candle`: benchmark both (p50/p99, binary size, musl/Win/macOS, ONNX fidelity, maintenance) in ADR before code.

- Enforce L2 <10ms p50/<25ms p99 in CI; fallback to L3 on missing/slow/corrupt model; musl static still works; binary-size measured.
- AC: parity gate passes; deleted/corrupt model → ask-via-L3 proven.

## Caveats (from §4.1, must resolve)

- ToS permission, consent + no unredacted secrets/code, L2 evaluated on same set as Jev.
