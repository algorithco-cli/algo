# Jev v0.1 measurement — `<region>` — `<yyyy-mm-dd>` (TEMPLATE — do not fill; copy per run)

> Emitter: `eval/jev_client/measure.py`. This template mirrors its exact output shape.
> JSON schema: `jev-v0.1-TEMPLATE.json`. Per-record rows live in the `.json` (hashes only).

Dataset `<basename e.g. all.jsonl>` sha256 `<sha256 hex>`; questions `<questions version e.g. questions-v0.1>`; provider `<server-echoed model id, e.g. jev-1.13.0>` (requested: `<model, e.g. jev-latest>`); runs: 3.

## Metrics (mean / min / max across runs; `stdev` in `.json` only = cross-run variance)

| metric | mean | min | max | budget |
|---|---|---|---|---|
| false_allow | TBD | TBD | TBD | gate metric (dangerous→allow) |
| false_ask | TBD | TBD | TBD |  |
| false_deny | TBD | TBD | TBD |  |
| ece | TBD | TBD | TBD |  |
| brier | TBD | TBD | TBD |  |
| lat_p50 | TBD | TBD | TBD | p50 < 250 ms |
| lat_p95 | TBD | TBD | TBD |  |
| lat_p99 | TBD | TBD | TBD | p99 < 800 ms |
| cost_per_1k_usd | TBD | TBD | TBD | list $0.042/Mtok input, output free (pinned per run) |
| error_timeout_ask_rate | TBD | TBD | TBD | must equal error rate; any `allow`-on-error = fail |

Gate: p50<250ms=TBD, p99<800ms=TBD. Per-record rows live in the .json (hashes only).

## Notes

- Payloads redacted/hashed only (`state_sha256`, never raw states). Cross-run variance = `min`/`max`/`stdev` per metric in the `.json`.
- Region = vantage label until vendor regions confirmed (`docs/verify/jev-api.md` §5); never present a vantage label as a vendor region.
- Miss vs budget ⇒ ADR, never silent adjustment.
