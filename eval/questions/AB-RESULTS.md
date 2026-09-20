# A/B plumbing result — baselines over full dataset v0.1 (`P0-EVAL-6` prep)

> Proves the `harness.cli` A/B plumbing on the full 240-record collection.
> Same slice, same harness version, two baseline providers.
> Date: 2026-09-20 | Dataset: `datasets/v0.1/seed.jsonl` v0.1
> (`sha256 18a3d49761187028d5889e60bea3e0dfb259577f0b1791a5bb12577fd4aaee8b`)

## Operating points (full 240: 96 SAFE / 72 DANGEROUS / 72 AMBIGUOUS)

| Provider (v0.1.0) | Accuracy | false_allow (rate over DANGEROUS) | ask_rate | ambiguous slice |
|---|---|---|---|---|
| `rules_only` | 0.425 | 66 (0.917) | 0.000 | 0/72 (0.000) |
| `mock_ask_all` | 0.300 | 0 (0.000) | 1.000 | 72/72 (1.000) |

Confusion (`rules_only`): SAFE 96 ALLOW; DANGEROUS 66 ALLOW / 6 DENY;
AMBIGUOUS 72 ALLOW. (`mock_ask_all`: everything ASK.)

## Δfalse-allow at fixed false-ask

- Raw delta (`rules_only` − `mock_ask_all`): **+66 false-allows**
  (rate **+0.917**).
- Fixed-false-ask caveat: the two baselines sit at opposite ask extremes
  (0.000 vs 1.000), so a same-ask delta is not directly readable from these
  two points — it needs the confidence-threshold sweep
  (`harness/threshold_sweep.py`, full version lands in `P1-QUAL`).
  What this run proves for `P0-EVAL-6`: identical-slice execution,
  pinned `report.json` artifacts (`dataset.{version,sha256}` +
  `provider.{name,version,cost_usd_per_decision}`), and comparable
  false-allow / ask-rate / ambiguous-accuracy columns to rank the
  `questions/v0.1.yaml` phrasing x batching cells against.
- Headroom confirmed: `rules_only` allows 66/72 dangerous and scores 0/72
  on the ambiguous slice — a real policy must beat its ambiguous accuracy
  at equal-or-lower false-allow (per `ci-eval.md` gate plumbing).

## Jev-provider A/B

- **Pending API key.** The Jev judgment-provider cell (same slice, same
  harness, `JEV` source level) has not been run — no key is configured in
  this environment. Re-run this file's table with the Jev provider once
  credentials land, and pin the winning
  (judgment, phrasing, batching) triple as `questions-v0.1` in `DATASET.md`.

## Repro

```powershell
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider rules_only --output reports/ab_rules_only
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/ab_mock_ask_all
```

(Run outputs under `eval/reports/` are git-ignored scratch; only this file
is kept.)
