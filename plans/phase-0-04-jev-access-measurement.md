# P0-04 Jev Verification + Measurement

## 1. API/SDK dossier (`P0-JEV-1`)

`docs/verify/jev-api.md`, one link+date per bullet: REST spec, auth, rate limits, zero-retention, regions, SDK (Go/Rust), on-prem/enterprise, status page.
Acceptance: every confirmed fact cited; gaps marked `[VERIFY-OPEN]` with owner + recheck date. Test client built only from spec.

## 2. ToS distillation check (`P0-JEV-2`, gates L2)

`docs/verify/jev-tos.md` with quoted section + verdict (allowed/conditional/prohibited; ticket ID if ambiguous).
Acceptance: cited verdict + human sign-off; if blocked, L2 replanned to consent-only data, D-A ADR updated.

## 3. Probe client (`P0-JEV-3`)

`eval/jev_client/` (env-only key, pre-send redaction, persistent HTTP/2, timeout, never logs key/payload) + `.env.example`.
Acceptance: ≤5 probe requests succeed; logs pass secret-scan; key never committed.

## 4. Measurement protocol (`P0-JEV-4`)

3 runs over full v0.1 from ≥2 regions; record false-allow/ask/deny, calibration, per-question + e2e latency vs §4.1 L3 (p50 <250ms, p99 <800ms), cost/1k, error/timeout→`ask` rate; redacted-only artifacts.
Acceptance: `eval/reports/jev-v0.1-<region>-<date>.md+json` with variance; payloads redacted/hashed. **Risk:** High — may fail gate.

## 5. Vendor claims

§1/§4.1 faster/cheaper claims replaced with measured numbers; no claim carried into docs without report link. L0–L4 traffic shares remain hypotheses, never hard-coded.
