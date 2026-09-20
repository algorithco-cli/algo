# Jev vendor-claims ledger (`P0-JEV-5` / plan §5)

Rule (master plan §1, §10.7): vendor faster/cheaper claims are HYPOTHESES until we
measure them. No claim is carried into product docs without a linked measurement
report (`eval/reports/jev-v0.1-<region>-<date>.md+json`). L0–L4 traffic shares are
hypotheses and are NEVER hard-coded (they are config/measured values only).

## Claim table (VENDOR CLAIM → MEASURED)

| # | Vendor claim (source, date) | Our MEASURED columns | Status |
|---|---|---|---|
| C-1 | "much faster and cheaper than LLMs for this kind of work" (master plan §1; vendor homepage "193.6x Faster, 444.6x Cheaper", https://typesafe.ai/ verified 2026-09-20) | p50/p95/p99 per-question + e2e latency: PENDING; cost/1k requests: PENDING | HYPOTHESIS — report: TBD (link when `eval/reports/jev-v0.1-*.md` exists) |
| C-2 | 70–500ms end-to-end; input $0.042/Mtok, output free (https://docs.typesafe.ai/models verified 2026-09-20) | p50 vs 250ms budget, p99 vs 800ms budget: PENDING; invoiced $/1k: PENDING | HYPOTHESIS — price confirmed as list price only; stability open (vendor's own "temporary or subsidized?" question) |
| C-3 | 13-question batching "12.2× cheaper and 10.0× faster with no change in answers" (https://docs.typesafe.ai/cookbooks/parallel_questions verified 2026-09-20) | batch-vs-single Δlatency, Δcost, Δfalse-allow (P0-EVAL-6 A/B): PENDING | HYPOTHESIS — our batching test owns this number |
| C-4 | Calibrated confidence usable for routing ("confidence tells you whether to act", https://docs.typesafe.ai/patterns/confidence-routing verified 2026-09-20) | ECE / Brier on v0.1: PENDING; threshold sweep per profile: PENDING | HYPOTHESIS — calibration is measured, never assumed |
| C-5 | "Most requests never reach Jev" via L0–L2 (master plan §4.1) | L0–L4 traffic shares: PENDING (Phase 1+ telemetry) | HYPOTHESIS — shares stay configurable; no hard-coded split |

## L3 budget (from master plan §4.1 — the bar measurement must clear)

| Path | p50 | p99 |
|---|---|---|
| L3 (Jev) | < 250 ms | < 800 ms |

Over-budget ⇒ no merge / ADR (gates doc §1). Probe/measure timeout default 800 ms
(= p99 budget) maps to `ask` per fail-safe rule.

## How a claim graduates

1. Run protocol in `eval/jev_client/README.md` (3 runs × ≥2 regions, full v0.1).
2. Fill `eval/reports/jev-v0.1-TEMPLATE.md+json` → publish dated report.
3. Link report in the Status column above; copy MEASURED numbers (never vendor
   numbers) into product docs.
4. If measured numbers miss budgets, file ADR — do not silently adjust.
