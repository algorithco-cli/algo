# Jev measurement unblock checklist (human-run, ordered)

No live Jev calls run until step 6. Key is env-only, never committed.
Region labels are vantage labels until vendor regions are confirmed (§3).

## 1. Procure key (env-only, owner: eval lead)

- Obtain org key for `algorithcoguard` via console (see `docs/verify/jev-api.md` §2).
  Record holder, rotation, spend cap, plan tier in the tracking issue. Do NOT
  paste the key into docs, chat, or any file.
- Expected artifact: key held by owner only; nothing committed.

```powershell
$env:ALGO_JEV_API_KEY = "<key-from-owner>"
$env:ALGO_JEV_REGION = "vantage-eu-central"
$env:ALGO_JEV_MODEL = "jev-latest"
```

```bash
export ALGO_JEV_API_KEY="<key-from-owner>"
export ALGO_JEV_REGION="vantage-eu-central"
export ALGO_JEV_MODEL="jev-latest"
```

- Verify: `eval/jev_client/.env.example` stays empty (`ALGO_JEV_API_KEY=` with no
  value); no `.env` file is committed (see `.gitignore`).

## 2. Sign ToS verdict + read AUP + file distillation ticket (owner: product/legal — gates L2)

- Read `docs/verify/jev-tos.md` Q1–Q4; sign the Human sign-off lines (reviewer +
  date, L2 contingency accepted OR enterprise path opened).
- Read the full AUP. WARNING 2026-09-20: `https://typesafe.ai/legal/aup`
  (http + https) returns 404 — locate the current AUP URL first, then confirm
  redacted dangerous-command eval traffic is acceptable use; record the URL +
  date in `jev-tos.md` [VERIFY-OPEN-1].
- Read the DPA (`https://typesafe.ai/legal/data-processing`, DPA last updated
  Apr 24 2026): retention, subprocessors, deletion SLA ([VERIFY-OPEN-2]).
- File the §2.3(b) distillation-exception ticket (Q4: "Does §2.3(b) prohibit
  training a small local safety classifier on Jev outputs … is an enterprise
  amendment available?"); record the ticket ID in `jev-tos.md` Q4
  ([VERIFY-OPEN-3]). Until answered in writing, the PROHIBITED verdict stands
  and L2 trains on consent-only data only.
- Expected artifacts: signed `jev-tos.md` sign-off, current AUP URL + verdict,
  ticket ID in Q4.

## 3. Confirm regions (owner: eval lead)

- Check `https://docs.typesafe.ai/llms.txt` index + `https://docs.typesafe.ai/models`
  for serving region(s)/residency; ask support/sales if absent
  (`docs/verify/jev-api.md` §5 [VERIFY-OPEN]).
- Until confirmed: `region` in reports = vantage network label (e.g.
  `vantage-eu-central`), explicitly NOT a vendor region. If the vendor confirms
  a single region, file an ADR updating the ≥2-region protocol before measuring.
- Expected artifact: region decision recorded (two vantage labels OR vendor
  regions + ADR if changed).

## 4. Pin questions (pending P0-EVAL-6 Jev A/B)

- `eval/jev_client/measure.py` currently uses `QUESTIONS_VERSION =
  "questions-v0.1-provisional"` (single `decision` choice over
  allow/ask/deny). Do NOT measure until P0-EVAL-6 pins `questions-v0.1`
  (including the batch-vs-single A/B that owns the 12.2×/10.0× claim).
- After pinning: update `QUESTIONS_VERSION`/`QUESTIONS` in `measure.py` to the
  pinned set and record the version string.
- Expected artifact: pinned `questions-v0.1` file + `QUESTIONS_VERSION` updated
  in `measure.py`.

## 5. Tag dataset `eval-data-v0.1` (P0-EVAL-3 schema)

- Build the full v0.1 dataset (`{"id","action","label","obfuscated"?}`), tag it
  `eval-data-v0.1`, and record its SHA:

```powershell
# workdir: eval/
python -m harness.cli --dataset datasets/v0.1/seed.jsonl --provider mock_ask_all --output reports/coverage --coverage-only
git tag eval-data-v0.1
(Get-FileHash datasets/v0.1/all.jsonl -Algorithm SHA256).Hash
```

- Expected artifacts: git tag `eval-data-v0.1`, dataset path
  `datasets/v0.1/all.jsonl`, SHA256 recorded (must match `dataset_sha256` in
  every report).

## 6. Run `probe.py` (≤5 requests)

```powershell
# workdir: eval/jev_client/
pip install -r requirements.txt
python probe.py
```

- Expected artifacts: exit 0; console shows `[1/5] models: [...]` + up to 3
  `action=… conf=… model=… latency=…ms` lines; `PROBE OK: N requests
  (limit <=5)`. Any `JevError` → exit 2 (ask-path works, access does not —
  fix key/config, do not proceed). Key/payloads never printed (hashes only).

## 7. Run `measure.py` — 3 runs × ≥2 regions

```powershell
# workdir: eval/jev_client/
python measure.py --dataset ../../datasets/v0.1/all.jsonl --regions vantage-eu-central vantage-us-east --runs 3 --out-dir ../reports
```

- Requires: `ALGO_JEV_API_KEY` set (else exit 2, no live calls made); `rune ≥2
  regions` enforced (else exit 2).
- Expected artifacts per region: `eval/reports/jev-v0.1-<region>-<date>.md` +
  `.json` matching `jev-v0.1-TEMPLATE.json` shape — `dataset`/`dataset_sha256`,
  `questions_version`, `provider`=`typesafe-jev` + server-echoed
  `provider_version`, `model_requested`, `region`, `date`, `runs`, pinned
  `list_price_per_mtok_usd`, `budgets` (L3 p50<250/p99<800), `metrics`
  (false_allow/false_ask/false_deny, ece/brier, lat_p50/p95/p99,
  cost_per_1k_usd, error_timeout_ask_rate — each mean/min/max/stdev),
  `gate` (p50_lt_250, p99_lt_800). Console prints per-region
  `false_allow=… p50=…ms p99=…ms` plus cross-region `false_allow` spread.
- Then: link each report in `docs/verify/jev-claims.md` Status column (measured
  numbers only, never vendor numbers); over-budget ⇒ ADR, never silent
  adjustment. Dated run outputs are git-ignored; only `README.md` +
  `jev-v0.1-TEMPLATE.*` are tracked.
