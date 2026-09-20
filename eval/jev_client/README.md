# `eval/jev_client/` — Jev probe + measurement client (Phase 0, `P0-JEV-3`/`P0-JEV-4`)

Throwaway-eligible probe code (per AGENTS.md Phase 0: only redacted probe client lives
here; no product code). Built ONLY from the public spec
(`docs/verify/jev-api.md`); no invented fields.

## Files

| File | Purpose |
|---|---|
| `client.py` | Typed client: env-only key, pre-send redaction hook, persistent HTTP/2, 800 ms timeout → ask, hashes-only logs |
| `redaction.py` | LOCAL STUB. Preferred order: `harness.redact` (P0-EVAL-4 canonical) → this stub. Replaced, not forked. |
| `probe.py` | Connectivity probe: **≤5 requests** (1× models + ≤4× evaluate). Exit 0 = access works. |
| `measure.py` | Full protocol runner: 3 runs × ≥2 regions → per-region `.md+json` reports. |
| `requirements.txt` | `httpx[http2]` only. Python ≥3.10. |
| `.env.example` | `ALGO_JEV_API_KEY=` (empty), `ALGO_JEV_REGION=`, `ALGO_JEV_MODEL=`. Never commit a filled copy. |

## Quick start (once a key exists — NO live calls without one)

```bash
pip install -r requirements.txt
export ALGO_JEV_API_KEY="..."          # PowerShell: $env:ALGO_JEV_API_KEY = "..."
export ALGO_JEV_REGION="vantage-eu-central"
python probe.py                        # <=5 requests; exit 0 iff access works
python measure.py --dataset ../../datasets/v0.1/all.jsonl \
    --regions vantage-eu-central vantage-us-east --runs 3 --out-dir ../reports
```

## Guarantees

- Key is env-only (`ALGO_JEV_API_KEY`); `JevConfigError` if unset — no live traffic possible.
- Every state passes the redaction hook before send; fixtures are pre-redacted anyway.
- Timeout default 800 ms (= L3 p99 budget) and ALL transport/parse/rate-limit failures
  raise `JevError`, which callers map to `ask` — never `allow` (fail-safe, master plan §2.1).
- Logs: sha256 hashes, counts, latencies, usage. The key and payloads are NEVER logged,
  printed, or stored — logs pass secret-scan; `tests/test_no_secrets.py` (P0-EVAL-1) covers this.
- Region labels are vantage labels until vendor regions are confirmed
  ([VERIFY-OPEN], `docs/verify/jev-api.md` §5); reports state this explicitly.

## Measurement protocol (P0-JEV-4, runnable via `measure.py`)

1. Prerequisites: dataset tag `eval-data-v0.1` (P0-EVAL-3), questions pinned
   (currently `questions-v0.1-provisional`; P0-EVAL-6 pins `questions-v0.1`), AUP recheck
   done (`docs/verify/jev-tos.md` [VERIFY-OPEN-1]).
2. Run 3 full passes per region, ≥2 regions, same dataset SHA + questions version.
3. Record per region: false-allow/ask/deny, ECE/Brier, per-question + e2e latency
   p50/p95/p99 vs L3 budgets (p50 <250 ms, p99 <800 ms), cost/1k, error/timeout→ask rate,
   dataset SHA, provider version (server-echoed id), region, date, variance stats.
4. Publish `eval/reports/jev-v0.1-<region>-<date>.md+json` (schema: `jev-v0.1-TEMPLATE.json`).
   Payloads redacted/hashed only. Link reports in `docs/verify/jev-claims.md`.
5. Gate: Phase-0 exit needs measured false-allow + latency; miss ⇒ redesign/ADR, not silence.
