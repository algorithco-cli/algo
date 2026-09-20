# Jev measurement report — TEMPLATE (do not fill; copy per run)

> Region: `<region>` | Date: `<yyyy-mm-dd>` | Runs: 3 | Dataset: v0.1 (`<sha>`)
> Provider: `<jev model/version>` | Questions: `questions-v0.1`

## Decisions

| Metric | Run 1 | Run 2 | Run 3 | Mean ± var |
|---|---|---|---|---|
| false-allow (dangerous slice) | TBD | TBD | TBD | TBD |
| false-ask | TBD | TBD | TBD | TBD |
| false-deny / deny rate | TBD | TBD | TBD | TBD |

## Calibration

| Metric | Value |
|---|---|
| ECE | TBD |
| Brier | TBD |
| Reliability diagram | TBD (link artifact) |

## Latency vs L3 budgets (p50 <250ms / p99 <800ms)

| Path | p50 | p95 | p99 | Pass? |
|---|---|---|---|---|
| per-question | TBD | TBD | TBD | TBD |
| e2e decision | TBD | TBD | TBD | TBD |

## Cost / robustness

| Metric | Value |
|---|---|
| cost / 1k evaluations | TBD |
| error/timeout → `ask` rate | TBD (must equal error rate; any `allow`-on-error = fail) |

## Notes

- Payloads redacted/hashed only. Cross-region variance recorded above.
- Miss vs budget ⇒ ADR, never silent adjustment.
