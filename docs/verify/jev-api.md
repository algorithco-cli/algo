# Jev API/SDK dossier (`P0-JEV-1`)

Vendor: TypeSafe AI, model Jev (System One model). Org: `algorithcoguard`, CLI `algo`.
Compiled 2026-09-20 from official docs. Rule: every confirmed fact carries link+date;
anything unconfirmed is `[VERIFY-OPEN]` with owner + recheck date.
Test/probe client (`eval/jev_client/`) is built ONLY from the spec below — no guessed fields.

## 1. REST spec

- Endpoint `POST https://api.typesafe.ai/v1/systemone` — request JSON
  `{state, model, questions}`, response `{model, answers, usage}`.
  Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- `state`: string | object | array (text only — pre-process images/audio/video/binaries
  into text first). Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- `questions`: map of caller-chosen id → typed `Question` (`noul` | `choice` | `score`).
  Question ids are NOT sent to the model and do not affect inference.
  Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- `noul`: yes/no, returns probability 0–1 (+ optional `criteria.true/false`).
  `choice`: max 255 options, returns `choice` + full `probabilities` + `confidence`.
  `score`: 2–10 ordered levels, returns probability-weighted `score` + `legend` +
  `probabilities` + `confidence`. Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- All questions in one call are evaluated in parallel against the same state;
  batching does not change answers (vendor cookbook reports 13-question batching
  12.2× cheaper / 10.0× faster — HYPOTHESIS until we measure, see `jev-claims.md`).
  Source: https://docs.typesafe.ai/cookbooks/parallel_questions (verified 2026-09-20).
- `GET /v1/models` lists usable model names/aliases with description + release date.
  Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- Current model `jev-1.13.0`; aliases `jev-latest` (stable default), `jev-preview`.
  Alias targets move on release; response `model` field reports the versioned id that
  answered — log it per request. Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- Context: 64k tokens/request total; 32k budget for `state` + longest question.
  Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- Errors: `401` bad/missing key; `422` validation (body names offending field);
  `429` rate limit; `529` overloaded. Retry 429/529 with exponential backoff;
  official SDKs do this by default and honor `retry-after`.
  Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- Pricing: $42/Btok = $0.042/Mtok input tokens; output tokens free.
  Source: https://docs.typesafe.ai/models (verified 2026-09-20).
  NOTE: vendor homepage asks "Are these prices temporary or subsidized?" as an open
  question — treat price as subject to change; pin the invoiced rate in each report.
  Source: https://typesafe.ai/ (verified 2026-09-20).
- English is the primary training language and most accurate; other languages incl. CJK
  work less reliably — test on own data. Source: https://docs.typesafe.ai/models
  (verified 2026-09-20). (Our eval dataset v0.1 is English shell-heavy; no action.)

## 2. Auth

- `Authorization: Bearer <API_KEY>` header; key via `TYPESAFE_API_KEY` env in official
  SDKs. Our client uses `ALGO_JEV_API_KEY` (env-only, never logged) and sends the same
  Bearer scheme. Source: https://docs.typesafe.ai/api (verified 2026-09-20).
- Access is waitlisted/early-access; keys via console; gateway resellers (OpenRouter,
  Vercel/Netlify AI Gateway, AIMLAPI) also serve Jev. Gateway use is NOT our plan
  (privacy + measurement cleanliness) — recorded here only to avoid confusion.
  Source: https://docs.typesafe.ai/models + third-party page (verified 2026-09-20).
- [VERIFY-OPEN] Org key procurement for `algorithcoguard` (who holds the key, rotation,
  spend cap, which plan tier). Owner: eval lead. Recheck: 2026-10-04.

## 3. Rate limits

- 250,000 tokens/sec + 1,200 requests/min; over-limit → `429`. SDKs retry with backoff
  and honor `retry-after`. Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- WARNING (vendor): "Rate limits are adjusting dynamically… can change without notice."
  Higher limits on custom/enterprise plans (`sales@typesafe.ai`).
  Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- [VERIFY-OPEN] Re-confirm limits at measurement time and log the effective limit in
  every report (limits may have moved). Owner: eval lead. Recheck: 2026-10-04
  (and at each measurement run).

## 4. Zero-retention / data handling (updated with owner-confirmed Privacy Policy + subprocessors 2026-09-20)

- "Jev is not trained on customer requests or responses."
  Source: https://docs.typesafe.ai/models ("Data handling", verified 2026-09-20).
- "Jev is not fine-tuned or LoRA-adapted with customer data… the same weights serve
  every account." Source: https://docs.typesafe.ai/models (verified 2026-09-20).
- Privacy Policy (https://typesafe.ai/legal/privacy-policy, Nov 19, 2025, fetched 2026-09-20, owner-confirmed): TypeSafe **will not train or fine-tune any AI/ML models on Input** (prompts/other Input) and **will not disclose Input to a third party other than service providers**.
- Zero data retention (ZDR) is ENTERPRISE-ONLY via `privacy@typesafe.ai`:
  "We also offer zero data retention (ZDR) for enterprise customers. Contact privacy@typesafe.ai."
  Source: https://docs.typesafe.ai/legal + Privacy Policy (verified 2026-09-20, owner-confirmed).
- MCA §4.3: vendor may process "Telemetry… without restriction, including to improve
  the Services" — see `jev-tos.md` (quoted, verified 2026-09-20).
- Retention: DPA Schedule I §8 "as long as necessary" + Privacy Policy "Retention" — **"for as long as reasonably necessary" with no fixed SLA** (owner-confirmed 2026-09-20). No deletion SLA is published.
- Hosting: **US** — Privacy Policy "International Visitors": Services are **hosted in the US**; non-US users transfer data to the US. All subprocessors USA (see below).
- Subprocessors (owner-confirmed 2026-09-20 from https://trust.typesafe.ai/subprocessors): **AWS** (stores live-request data), **Modal / Nebius / CoreWeave** (process, do **not** store), **Slack + Google Workspace** (support) — **all USA**.
- [VERIFY-OPEN] Whether Telemetry collection is opt-out-able on standard plans beyond the Privacy Policy training/disclosure limits above.
  Owner: product/legal. Recheck: 2026-10-20.
- [VERIFY-OPEN] SOC2/ISO or equivalent certifications. Owner: product/legal.
  Recheck: 2026-10-20.

## 5. Regions / data residency (owner-confirmed: hosted US, all subprocessors USA)

- **Hosted US** — Privacy Policy "International Visitors" (Nov 19, 2025, fetched 2026-09-20, owner-confirmed): Services are hosted in the **US**; non-US users transfer data to the US. All subprocessors above are **USA** (AWS / Modal / Nebius / CoreWeave / Slack / Google Workspace).
- [VERIFY-OPEN] Serving region(s), data residency options beyond US hosting, and whether the endpoint or
  response headers expose region. Nothing on regions found in
  https://docs.typesafe.ai/llms.txt index (checked 2026-09-20).
  Owner: eval lead. Recheck: 2026-10-04.
- Consequence: P0-JEV-4 requires ≥2 regions. Until vendor residency options beyond US are confirmed, "region"
  in reports = the vantage network used for the run (labeled as such, NOT as a vendor
  region). If vendor confirms a single region, file an ADR updating the protocol.

## 6. SDK (Go/Rust) — DECISION D-02 input

- Official SDKs exist for Python and JavaScript/TypeScript ONLY.
  Source: https://docs.typesafe.ai/sdk (verified 2026-09-20).
- There is NO official Go or Rust SDK (docs index lists no Go/Rust SDK pages;
  checked https://docs.typesafe.ai/llms.txt 2026-09-20).
- Therefore the `agent` Jev client (Rust: hyper/reqwest, HTTP/2 + pooling per master
  plan §5.3) MUST be a small typed client built from the spec in §1 above — exactly
  what the plan anticipates ("otherwise build a small typed client from the official
  API spec"). No SDK behavior (retry defaults, env names) may be assumed; only the
  documented HTTP contract.
- [VERIFY-OPEN] Recheck for a first-party Go/Rust SDK quarterly before building new
  client features. Owner: agent lead. Recheck: 2026-12-20.

## 7. On-prem / enterprise

- Enterprise plans exist (higher rate limits, ZDR). Contact `sales@typesafe.ai` /
  `privacy@typesafe.ai`. Source: https://docs.typesafe.ai/models + https://docs.typesafe.ai/legal
  (verified 2026-09-20).
- [VERIFY-OPEN] On-prem / VPC / dedicated-tenancy option: not mentioned in docs.
  Owner: product. Recheck: 2026-10-20.
- [VERIFY-OPEN] Enterprise amendment path for the distillation ban (MCA §2.3(b)) —
  gates L2; see `jev-tos.md`. Owner: product/legal. Recheck: 2026-10-20.

## 8. Status page / SLA

- [VERIFY-OPEN] Status page URL, uptime SLA, support channel SLA (`support@typesafe.ai`
  is documented for Support — https://typesafe.ai/legal/mca §3, verified 2026-09-20 —
  but no SLA text found). Owner: eval lead. Recheck: 2026-10-04.

## 9. Acceptable use (pointer — does not exist, per owner)

- MCA §2.3(l) cites "TypeSafe's Acceptable Use Policy (located at typesafe.ai/legal/aup)"
  (verified 2026-09-20). **Owner-confirmed 2026-09-20:** legal index at `https://docs.typesafe.ai/legal`
  lists **only DPA, MCA, Privacy Policy — no AUP exists**; `https://typesafe.ai/legal/aup`
  returns **404** (https + http, re-verified 2026-09-20 for this update; MCA §2.3(l) is a
  **dangling reference**). [Prior hunt 2026-09-20: 3 search-engine sweeps + `trust.typesafe.ai` check also found no AUP.]
- [VERIFY-OPEN] Request **written confirmation from TypeSafe that no AUP exists** (email/ticket ID + date in `jev-tos.md:[VERIFY-OPEN-1]`). Per owner, this is **not a blocker** for eval/shadow traffic.
  Owner: product/legal. Recheck: written confirmation, not a URL search.
