# Jev Terms-of-Service distillation check (`P0-JEV-2`) — GATES L2

Source document: TypeSafe AI Master Customer Agreement (MCA),
https://typesafe.ai/legal/mca — "Last updated Sep 19, 2026" (fetched 2026-09-20).
Quotes below are verbatim excerpts used under fair-use for compliance review.
AUP: referenced but not yet read — see [VERIFY-OPEN-1].

## Quoted sections + verdicts

### Q1 — MCA §2.3(b) (License Restrictions)

> "Customer will not do (and will not attempt to do)… any of the following:
> … (b) use the Services or any Output (defined below) to perform model
> distillation, train a model to imitate the output of the Services, or develop
> (or to facilitate the development of) a similar or competing product or service"

Verdict: **PROHIBITED** — training the L2 local classifier on Jev outputs
(master plan §4.1: L2 "trained from Jev outputs") is model distillation as defined
here and is banned under the standard MCA. This verdict is cited, not ambiguous:
the clause names distillation explicitly.

### Q2 — MCA §4.2 (Output ownership)

> "As between Customer and TypeSafe and to the extent permitted by Laws, TypeSafe
> does not claim ownership of Input and TypeSafe disclaims ownership of Output.
> TypeSafe hereby assigns to Customer all of its right, title, and interest, if any,
> in the Output."

Verdict: **ALLOWED** — using Jev answers at runtime (L3 decisions, audit log,
threshold tuning on our own labels) is permitted; we own the Output. Ownership does
NOT override §2.3(b): owned output still may not be used for distillation.

### Q3 — MCA §4.1 (Use of Customer Data) + §4.3 (Telemetry)

> §4.1: "…in perpetuity, any Customer Data (i) to derive and generate Telemetry,
> (ii) to monitor for fraud and abuse of the Services, and (iii) as necessary to
> comply with applicable Laws. The foregoing license does not grant TypeSafe the
> right to, and TypeSafe will not, include Customer Data in a dataset used to train
> (i.e., to modify the model weights of) any artificial intelligence or machine
> learning models without Customer's prior consent."
> §4.3: "'Telemetry' means information generated in connection with the Services,
> such as technical logs, hashes, summary statistics and classifications, metrics,
> and learnings related to Customer's use of the Services. TypeSafe may Process
> Telemetry without restriction, including to improve the Services or TypeSafe's
> other products and services."

Verdict: **CONDITIONAL** — standard-plan traffic leaves the machine under a
perpetual telemetry license (hashes, summary statistics, "learnings related to
Customer's use"). Allowed ONLY with: (a) pre-send redaction (no secrets/source
beyond what the user approved), (b) `redacted` default / `local-only` mode per UX
checklist, (c) ZDR enterprise tier pursued for team traffic. Weight-training on our
data requires our prior consent (allowed to withhold).

### Q4 — Vendor cookbook vs MCA (ambiguity → ticket)

The official AutoResearch cookbook trains a downstream classical (CatBoost) model on
Jev probabilities (https://docs.typesafe.ai/cookbooks/autoresearch_feature_discovery,
verified 2026-09-20), which reads in tension with §2.3(b).
Verdict: **CONDITIONAL / AMBIGUOUS** — ticket to TypeSafe support/sales:
"Does §2.3(b) prohibit training a small local safety classifier on Jev outputs for
use inside our own product, and is an enterprise amendment available?"
Ticket ID: TBD (file before any L2 training; record ID here).
Until answered in writing: the PROHIBITED verdict (Q1) stands.

### Verdict summary

| Use | Verdict | Basis |
|---|---|---|
| L3 runtime judgments (allow/ask/deny) | allowed | MCA §2.1 license + §4.2 |
| Threshold tuning on OUR labels | allowed | §4.2; no Jev-output training |
| L2 classifier trained on Jev outputs | **prohibited** (standard MCA) | §2.3(b), quoted above |
| L2 enterprise exception | ambiguous → ticket TBD | cookbook vs §2.3(b) tension |
| Sending redacted states to Jev | conditional (redact + consent + ZDR path) | §4.1/§4.3 |

## L2-blocking note

L2 is BLOCKED on standard terms. Contingency (no ADR delay to Phase 0):
**L2 replanned to consent-only data** — user confirmations + hand labels only, zero
Jev outputs in training — and D-A ADR (local-model runtime, ort vs candle) updated
to record the data-source restriction. If the ticket returns written clearance or an
enterprise amendment, a follow-up ADR re-enables Jev-distilled training. Either way,
L2 ships only if it meets the same eval threshold as Jev on its traffic share
(master plan §4.1 caveat, unchanged).

## [VERIFY-OPEN] items

- [VERIFY-OPEN-1] Read full AUP (`typesafe.ai/legal/aup`) — confirm redacted
  dangerous-command eval traffic is acceptable use. Owner: product/legal.
  Recheck: 2026-10-04 (before live measurement).
- [VERIFY-OPEN-2] Read DPA (`typesafe.ai/legal/data-processing`) — retention,
  subprocessors, deletion. Owner: product/legal. Recheck: 2026-10-20.
- [VERIFY-OPEN-3] Distillation exception ticket (see Q4) — file + record ID.
  Owner: product. Recheck: 2026-10-20.

## Human sign-off

- [ ] Reviewer (CODEOWNERS, legal/product): __________ Date: __________
- [ ] L2 contingency accepted (consent-only data) OR enterprise path opened: __________
- Gate effect: L2 work (Phase 3) may not consume Jev outputs until this sign-off
  plus either ticket clearance or the consent-only ADR update exists.
