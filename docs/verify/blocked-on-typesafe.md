# Blocked on TypeSafe — answered vs remaining (as of 2026-09-20 + MCA Sep 19, 2026)

> Source: MCA `https://typesafe.ai/legal/mca` (Sep 19, 2026, fetched 2026-09-20),
> DPA `https://typesafe.ai/data-processing` (Apr 24, 2026, fetched 2026-09-20),
> AUP `https://typesafe.ai/legal/aup` (404 both schemes, 2026-09-20).
> All sign-off fields blank — human act.

## Answered by the MCA/DPA (no longer blocked)

| # | Item | MCA/DPA clause | Answer |
|---|---|---|---|
| A1 | L2 distillation from Jev outputs | **MCA §2.3(b)** `docs/verify/jev-tos.md:Q1` | **PROHIBITED unless written exception** — train only on human labels / deterministic rules / user confirmations (`plans/phase-3-l2-model.md:1`). No dataset label or training feature may be derived from a Jev output. |
| A2 | API key confidentiality / embedded key | **MCA §2.4** `jev-tos.md:Q2` | **Confidential, no sharing, no embedding** in distributed binaries — BYOK default, no `TYPESAFE_API_KEY` literal in artifacts (`docs/adr/0004-byok-vs-proxy.md`). |
| A3 | Security/vuln testing of TypeSafe Services | **MCA §2.3(g)** `jev-tos.md:Q1b` | **Prohibited** — do not pentest `api.typesafe.ai`; our eval is of our policy/dataset only. |
| A4 | Customer obligations / consent basis | **MCA §5** `jev-tos.md:Q4` | **Consent required** to send any user data — grounds Phase 1 redact + `local-only` default + opt-in (`docs/redact-consent-readiness.md:2`). |
| A5 | Telemetry scope + perpetual license | **MCA §4.1/§4.3** `jev-tos.md:Q3` | Perpetual telemetry license on Customer Data (hashes/stats/learnings); TypeSafe will not train model weights without prior consent (we may withhold). Grounds `redacted` default + ZDR pursuit. |
| A6 | Indemnity risk (Input / End User) | **MCA §§12.3, 13.2** `jev-tos.md:Q5` | **Uncapped** for breach of §2.3/§2.4/§5 and for Input/End User claims — reinforces redact + consent + allow-list. |
| A7 | Branding / "powered by Jev" | **MCA §16.4** `jev-tos.md:Q6` | **Needs prior consent** — do not publish "powered by Jev" / logo / announcement without TypeSafe written consent. See flagged wordings below. |
| A8 | DPA processor duties, subprocessors, transfers, incident/audit | **DPA Apr 24, 2026 §§1–6 + Schedule I** `jev-tos.md:DPA summary` | Roles (controller/processor), subprocessor list at `trust.typesafe.ai/subprocessors` with 15-day objection, EU SCCs/UK Addendum, 72-hour incident notice, annual audit. |
| A9 | DPA location | **https://typesafe.ai/data-processing** (Apr 24, 2026) | Fetch and summarize in `jev-tos.md` DPA summary. |

## Remaining — still blocked / needs follow-up

| # | Item | Owner | What stays blocked | Follow-up ask | Priority |
|---|---|---|---|---|---|
| R1 | **AUP full read** — `typesafe.ai/legal/aup` is **404** (MCA §2.3(l) dangling; re-fetched for this update) — `jev-tos.md:[VERIFY-OPEN-1]`, `jev-api.md:134-145` | product/legal | Sending redacted dangerous commands to Jev (eval + shadow) may violate AUP; no further live measurement beyond the 3×240 already run without AUP | Locate current AUP URL, confirm redacted dangerous-command eval + shadow traffic is acceptable use; record URL+date in `jev-tos.md` | **P0** |
| R2 | **DPA open items** — fixed retention/deletion SLA, subprocessors list currency, whether Telemetry opt-out / ZDR avoids the §4.1 perpetual license — `jev-tos.md:[VERIFY-OPEN-2]`, `jev-api.md:80-86`, DPA Schedule I §8 "as long as necessary" (no fixed days) | product/legal | Retention/SLA uncertainty affects `redacted` shadow + any real-session data; audit scope | Confirm fixed retention days, deletion SLA post-termination, `trust.typesafe.ai/subprocessors` currency, Telemetry opt-out / ZDR need with `privacy@`/`sales@` | **P0** |
| R3 | **Distillation exception** — MCA §2.3(b) written exception for a small local safety classifier — `jev-tos.md:[VERIFY-OPEN-3]`, `jev-tos.md:Q7` | product | L2 Jev-distilled training stays **PROHIBITED**; human-only redesign is controlling (`plans/phase-3-l2-model.md:1`) | File ticket per Q7 wording, record ID; until written clearance, no Jev-derived training log | **P1** (gates L2, not Phase 1 auto-approve) |
| R4 | **Data residency / ZDR confirmation** — serving regions, residency, Telemetry opt-out vs ZDR enterprise — `jev-api.md:90-96,83-86` | product/legal + eval | Current `vantage-*` labels are not vendor regions; cannot promise residency; `redacted` mode still carries telemetry | Confirm regions/residency, ZDR enterprise path with `sales@`/`privacy@` | **P1** |
| R5 | **Branding consent** — "powered by Jev" etc. — `jev-tos.md:Q6`, MCA §16.4 | product/legal | Any public "powered by Jev" / logo / announcement | Request written consent if/when branding is desired; otherwise use neutral wording | **P2** (at launch) |
| R6 | **Proxy mode through our servers** — BYOK is default; proxy blocked until legal review — `docs/adr/0004-byok-vs-proxy.md` | backend/legal | Proxy design/build, per-org quota, abuse limits, audit, DPA §3 subprocessor implications | Legal review of proxy auth + billing + key custody (§2.4) before building | **P3** |

## How this doc relates to the other blockers list

- `docs/redact-consent-readiness.md:4` — Phase 1 shadow blockers (same R1/R2/R4 + redact/consent implementation gaps). That table is the **Phase 1 gate**; this doc is the **TypeSafe-answer inventory** (answered A1–A9 vs remaining R1–R6).
- `docs/exit-gate-P0.md:158` + `docs/verify/jev-api.md` [VERIFY-OPEN]s — hook/matrix + API dossier opens that are not TypeSafe-contract-driven (e.g., `docs/verify/jev-api.md:90-110` regions/SDK) are separate.

## Sign-off (leave blank — human act)

- [ ] A1–A9 answered items reviewed: __________ Date: __________
- [ ] R1–R6 remaining items prioritized + owners assigned: __________ Date: __________
- [ ] AUP relocation / DPA confirmation follow-up scheduled: __________ Date: __________
