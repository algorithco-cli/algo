# Jev vendor-claims ledger (`P0-JEV-5` / plan §5)

Rule (master plan §1, §10.7): vendor faster/cheaper claims are HYPOTHESES until we
measure them. No claim is carried into product docs without a linked measurement
report (`eval/reports/jev-v0.1-<region>-<date>.md+json`). L0–L4 traffic shares are
hypotheses and are NEVER hard-coded (they are config/measured values only).

## Claim table (VENDOR CLAIM → MEASURED)

| # | Vendor claim (source, date) | Our MEASURED columns | Status |
|---|---|---|---|
| C-1 | "much faster and cheaper than LLMs for this kind of work" (master plan §1; vendor homepage "193.6x Faster, 444.6x Cheaper", https://typesafe.ai/ verified 2026-09-20) | 2026-09-20 provisional `decision` single-question: eu-central p50 399ms p99 610ms; us-east p50 465ms p99 648ms; cost $0.0150/1k (see `eval/reports/jev-v0.1-vantage-*-2026-09-20.json`). **Latency facts added; cheaper/faster vs LLM still unproven (no LLM baseline).** | MEASURED (provisional questions) — reports: `eval/reports/jev-v0.1-vantage-eu-central-2026-09-20.md+json`, `eval/reports/jev-v0.1-vantage-us-east-2026-09-20.md+json` (3 runs × 240, `jev-1.13.0`, sha `18a3d49…ee8b`) |
| C-2 | 70–500ms end-to-end; input $0.042/Mtok, output free (https://docs.typesafe.ai/models verified 2026-09-20) | **Measured:** p50 399/465ms vs 250ms budget → **over budget**; p99 610/648ms vs 800ms → pass; list-price cost $0.0150/1k (see reports above). | MEASURED — p50 over budget triggers ADR-0008 (shadow-only, budget stays). Invoiced price still TBD (list price only confirmed). |
| C-3 | 13-question batching "12.2× cheaper and 10.0× faster with no change in answers" (https://docs.typesafe.ai/cookbooks/parallel_questions verified 2026-09-20) | batch-vs-single Δlatency, Δcost, Δfalse-allow (P0-EVAL-6 A/B): PENDING | HYPOTHESIS — our batching test owns this number |
| C-4 | Calibrated confidence usable for routing ("confidence tells you whether to act", https://docs.typesafe.ai/patterns/confidence-routing verified 2026-09-20) | ECE 0.560/0.561, Brier 0.524/0.525 (both vantages, 2026-09-20, provisional questions; `false_ask 1.0` signals uncalibrated on current phrasing). Threshold sweep: PENDING (EVAL-6). | MEASURED (provisional) — reports above; high ECE/Brier shows current phrasing not yet usable for routing — will re-measure after tuning. |
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
