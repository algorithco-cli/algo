# Blocked on TypeSafe — answered vs remaining (as of 2026-09-20 + MCA Sep 19, 2026)

> **Role note (2026-09-23 docs consolidation):** this file is the **dated research
> log** (evidence + owner confirmations as of 2026-09-20). The **single live
> tracker** for every open TypeSafe/Jev blocker (R1–R6 below) is
> [`docs/DEFERRED.md`](../DEFERRED.md) §4, which maps each item to the milestone
> it must be done BEFORE. Status changes go to DEFERRED.md; this file keeps its
> 2026-09-20 answers intact for the audit trail.

> Source: MCA `https://typesafe.ai/legal/mca` (Sep 19, 2026, fetched 2026-09-20),
> DPA `https://typesafe.ai/data-processing` (Apr 24, 2026, fetched 2026-09-20),
> Privacy Policy `https://typesafe.ai/legal/privacy-policy` (Nov 19, 2025, fetched 2026-09-20),
> owner-confirmed facts 2026-09-20 (see below).
> All sign-off fields blank — human act.

## Answered by the MCA/DPA/Privacy Policy + owner confirmation (no longer blocked)

| # | Item | Clause / source | Answer (owner-confirmed 2026-09-20) |
|---|---|---|---|
| A1 | L2 distillation from Jev outputs | **MCA §2.3(b)** `docs/verify/jev-tos.md:Q1` | **PROHIBITED unless written exception** — train only on human labels / deterministic rules / user confirmations (`plans/phase-3-l2-model.md:1`). No dataset label or training feature may be derived from a Jev output. |
| A2 | API key confidentiality / embedded key | **MCA §2.4** `jev-tos.md:Q2` | **Confidential, no sharing, no embedding** in distributed binaries — BYOK default, Jev off by default, no `TYPESAFE_API_KEY` literal in artifacts (`docs/adr/0004-byok-vs-proxy.md`). |
| A3 | Security/vuln testing of TypeSafe Services | **MCA §2.3(g)** `jev-tos.md:Q1b` | **Prohibited** — do not pentest `api.typesafe.ai`; our eval is of our policy/dataset only. |
| A4 | Customer obligations / consent basis | **MCA §5** `jev-tos.md:Q4` | **Consent required** to send any user data — grounds Phase 1 redact + `local-only` default + opt-in (`docs/redact-consent-readiness.md:2`, `docs/privacy-dataflow.md` consent draft). |
| A5 | Telemetry scope + perpetual license | **MCA §4.1/§4.3** `jev-tos.md:Q3` | Perpetual telemetry license on Customer Data (hashes/stats/learnings); TypeSafe will not train model weights without prior consent (we may withhold). Grounds `redacted` default + ZDR pursuit. |
| A6 | Indemnity risk (Input / End User) | **MCA §§12.3, 13.2** `jev-tos.md:Q5` | **Uncapped** for breach of §2.3/§2.4/§5 and for Input/End User claims — reinforces redact + consent + allow-list. |
| A7 | Branding / "powered by Jev" | **MCA §16.4** `jev-tos.md:Q6` | **Needs prior consent** — do not publish "powered by Jev" / logo / announcement without TypeSafe written consent. See `docs/verify/distillation-wording-flags.md`. |
| A8 | DPA processor duties, subprocessors (incl. confirmed list), transfers, incident/audit | **DPA Apr 24, 2026 §§1–6 + Schedule I**, owner-confirmed 2026-09-20 from `trust.typesafe.ai/subprocessors` + Privacy Policy | Roles (controller/processor), subprocessors **AWS** (stores live-request data) / **Modal, Nebius, CoreWeave** (process, do not store) / **Slack, Google Workspace** (support) — **all USA**, hosted **US** (Privacy Policy "International Visitors"); EU SCCs/UK Addendum, 15-day subprocessor objection, 72-hour incident notice, annual audit. |
| A9 | DPA + Privacy Policy locations | **DPA** `https://typesafe.ai/data-processing` (Apr 24, 2026); **Privacy Policy** `https://typesafe.ai/legal/privacy-policy` (Nov 19, 2025) | Fetched + summarized in `jev-tos.md` DPA/Privacy summaries. |
| A10 | **AUP — does not exist** | **Owner-confirmed 2026-09-20:** legal index lists **only DPA, MCA, Privacy Policy**; `typesafe.ai/legal/aup` 404 (https+http, re-fetched for this update; MCA §2.3(l) dangling) | **No AUP exists** — not a blocker. Action moved to written confirmation: request **written confirmation from TypeSafe that no AUP exists** (see R1). |
| A11 | **No training on Input; disclosure only to service providers** | **Privacy Policy "Services"** (Nov 19, 2025, owner-confirmed, fetched 2026-09-20) | TypeSafe **will not train or fine-tune AI/ML models on Input** (prompts/other Input) and **will not disclose Input to a third party other than service providers** — alongside MCA §4.1 carve-out. |
| A12 | **Retention + hosting + ZDR** | **Privacy Policy "Retention" + "International Visitors"** (Nov 19, 2025, owner-confirmed) | Retention is **"as long as reasonably necessary"** (Privacy Policy) / "as long as necessary" (DPA Schedule I §8) — **no fixed SLA**; hosted **US**; non-US users **transfer to US**; ZDR is **enterprise-only via `privacy@typesafe.ai`**. |

## Remaining — still needs follow-up

| # | Item | Owner | What stays blocked / needs action | Follow-up ask | Priority |
|---|---|---|---|---|---|
| R1 | **AUP written confirmation** — no AUP exists (A10), but MCA §2.3(l) still cites `typesafe.ai/legal/aup` | product/legal | Confirm the dangling reference is harmless; no AUP traffic rule to violate | Request **written confirmation from TypeSafe that no AUP exists** (email/ticket ID + date in `jev-tos.md:[VERIFY-OPEN-1]`); **not a blocker** per owner, but record the confirmation | **P1** (was P0, downgraded per owner) |
| R2 | **Fixed retention / deletion SLA beyond "as long as reasonably necessary"** — `jev-tos.md:[VERIFY-OPEN-2]`, `jev-api.md:80-86`, DPA + Privacy Policy | product/legal | **RESOLVED 2026-09-23 (owner decision):** TypeSafe states customer request/response data is not retained — no fixed retention/deletion SLA needed beyond that statement. Citation: **owner-asserted — vendor link TBD** ([VERIFY-OPEN]: file the TypeSafe URL + fetch date in `jev-tos.md:[VERIFY-OPEN-2]`; live vendor text checked 2026-09-23 still reads "as long as reasonably necessary" — Privacy Policy Retention Nov 19, 2025). Consent drafts no longer need to state "unspecified" retention on this basis | No further ask on retention; vendor-link filing only (recheck when link filed) | **RESOLVED** (was P1) |
| R3 | **Distillation exception** — MCA §2.3(b) written exception for a small local safety classifier — `jev-tos.md:[VERIFY-OPEN-3]`, `jev-tos.md:Q7` | product | L2 Jev-distilled training stays **PROHIBITED**; human-only redesign is controlling (`plans/phase-3-l2-model.md:1`) | File ticket per Q7 wording, record ID; until written clearance, no Jev-derived training log | **P1** (gates L2, not Phase 1 auto-approve) |
| R4 | **Data residency / regions confirmation** — `jev-api.md:90-96,83-86` | product/legal + eval | Current `vantage-*` labels are not vendor regions; all subprocessors USA per owner (US hosting) | Confirm regions/residency with `privacy@`/`sales@`; no SLA to promise users until answered | **P1** |
| R5 | **Branding consent** — "powered by Jev" etc. — `jev-tos.md:Q6`, MCA §16.4 | product/legal | Any public "powered by Jev" / logo / announcement | Request written consent if/when branding is desired; otherwise use neutral "Jev-compatible (BYOK)" wording | **P2** (at launch) |
| R6 | **Proxy mode through our servers** — BYOK is only real-data mode; proxy blocked until legal review — `docs/adr/0004-byok-vs-proxy.md` | backend/legal | Proxy design/build, per-org quota, abuse limits, audit, DPA §3 subprocessor implications | Legal review of proxy auth + billing + key custody (§2.4) before building | **P3** |

## How this doc relates to the other blockers list

- `docs/redact-consent-readiness.md:4` — Phase 1 shadow blockers (same R1/R2/R4 + redact/consent implementation gaps). That table is the **Phase 1 gate**; this doc is the **TypeSafe-answer inventory** (answered A1–A9 vs remaining R1–R6).
- `docs/exit-gate-P0.md:158` + `docs/verify/jev-api.md` [VERIFY-OPEN]s — hook/matrix + API dossier opens that are not TypeSafe-contract-driven (e.g., `docs/verify/jev-api.md:90-110` regions/SDK) are separate.

## Sign-off (leave blank — human act)

- [ ] A1–A9 answered items reviewed: __________ Date: __________
- [ ] R1–R6 remaining items prioritized + owners assigned: __________ Date: __________
- [ ] AUP relocation / DPA confirmation follow-up scheduled: __________ Date: __________
