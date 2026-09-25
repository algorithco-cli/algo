# A/B plumbing result — baselines over full dataset v0.1 (`P0-EVAL-6` prep)

> Same slice, same harness version, two baseline providers.
> Date: 2026-09-20 | Dataset: `datasets/v0.1/seed.jsonl` v0.1
> (`sha256 18a3d49761187028d5889e60bea3e0dfb259577f0b1791a5bb12577fd4aaee8b`)
> Updated 2026-09-20: `rules_only` is now realistic v0.2.0 (deny-list + heuristics, threat-model derived, not dataset-tuned).

## Operating points (full 240: 96 SAFE / 72 DANGEROUS / 72 AMBIGUOUS)

| Provider | Accuracy | false_allow (rate over DANGEROUS) | ask_rate | ambiguous slice | ECE | Brier |
|---|---|---|---|---|---|---|
| `rules_only` **v0.2.0** | 0.500 | 40 (0.556) | 0.108 | 9/72 (0.125) | 0.173 | 0.258 |
| `rules_only` v0.1.0 (retired stub, 3 patterns) | 0.425 | 66 (0.917) | 0.000 | 0/72 (0.000) | 0.188 | 0.271 |
| `mock_ask_all` | 0.300 | 0 (0.000) | 1.000 | 72/72 (1.000) | 0.200 | 0.250 |

Confusion **v0.2.0** (`rules_only`): SAFE 92 ALLOW / 4 ASK; DANGEROUS 40 ALLOW / 19 DENY / 13 ASK;
AMBIGUOUS 63 ALLOW / 9 ASK. (v0.1.0: SAFE 96 ALLOW; DANGEROUS 66 ALLOW / 6 DENY; AMBIGUOUS 72 ALLOW.)
(`mock_ask_all`: everything ASK.)

## Δfalse-allow at fixed false-ask

- Realistic baseline is now **v0.2.0** (0.556 false_allow). Raw delta vs always-ask (v0.2.0 − `mock_ask_all`): **+40 false-allows** (rate **+0.556**) at ask 0.108 vs 1.000 — opposite extremes, so same-ask delta is not directly readable; it needs the confidence-threshold sweep (`harness/threshold_sweep.py`, full version lands in `P1-QUAL`).
- What this run proves for `P0-EVAL-6`: identical-slice execution,
  pinned `report.json` artifacts (`dataset.{version,sha256}` +
  `provider.{name,version,cost_usd_per_decision}`), and comparable
  false-allow / ask-rate / ambiguous-accuracy / AUROC columns to rank the
  `questions/v0.1.yaml` phrasing x batching cells against the **realistic** baseline.
- Headroom: realistic `rules_only` still allows 40/72 dangerous — Jev must beat its ambiguous accuracy (0.125) and overall accuracy (0.500) at equal-or-lower false-allow to justify itself (per `ci-eval.md` gate plumbing at comparable ask).

## Jev-provider provisional (2026-09-20, vantage labels, single `decision` question)

- Measured 2026-09-20: both vantages **false_allow 0.000**, **false_ask 1.000**, p50 399/465ms p99 610/648ms, ECE 0.56 Brier 0.52 — **G2 not yet shown**: same-ask comparison vs realistic baseline requires the threshold sweep in `P1-QUAL`; raw point is not at comparable ask. Jev A/B over the threshold sweep and the pinned `questions-v0.1` triple will re-evaluate this section.
- Winner pinning (`DATASET.md`) awaits tuned questions.

## Repro

```powershell
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider rules_only --output reports/ab_rules_only
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/ab_mock_ask_all
```

(Run outputs under `eval/reports/` are git-ignored scratch; only this file
is kept.)

## Offline addendum 2026-09-24 (no Jev key; baselines only, combined 468)

- Combined set: seed 240 + expansion-A1 228 = 468 (SAFE 96 / DANGEROUS 300 /
  AMBIGUOUS 72). Dangerous n now meets the ~299 balanced minimum: with 0
  observed false_allow, exact Clopper-Pearson 95% upper = 0.0099 (<= 1.0%;
  Wilson 0.0126) — computed by `harness/bounds.py` (exact-integer cross-checked).
  Strict (~997) still needs Option B; fast (~149) is covered by count.
- `rules_only` v0.2.0 on combined: acc 0.404, false_allow 195/300 (0.650),
  ask 0.064, ambiguous_acc 0.125. Sweep (new `sweep_thresholds`): t=0.70
  demotes to false_allow 0 at ask 0.812 — i.e. the baseline can clear every
  false-allow ceiling, but only by asking 81% of the time (utility
  fails there: safe-slice ask would far exceed 0.30). This is the headroom Jev
  must beat: same-ask delta still requires live Jev confidences (measure.py now
  retains per-record rows for future runs).
- G2 status unchanged: opposite extremes (baseline ask 0.064-0.812 vs
  mock_ask_all 1.000) — no same-ask delta readable without Jev data.
  36/39 EVAL-6 cells remain blocked on `ALGO_JEV_API_KEY` plus 2 regions.

```powershell
python -m harness.cli --dataset reports/combined-468.jsonl --provider rules_only --output reports/combined-rules --sweep
```
