# Jev Terms-of-Service review — Master Customer Agreement (Sep 19, 2026)

> Source: TypeSafe AI Master Customer Agreement (MCA), https://typesafe.ai/legal/mca
> "Last updated Sep 19, 2026" (fetched 2026-09-20 + re-fetched for this update).
> DPA: https://typesafe.ai/data-processing — "Last updated Apr 24, 2026"
> (fetched 2026-09-20, summarized below).
> Privacy Policy: https://typesafe.ai/legal/privacy-policy — "Last updated Nov 19, 2025"
> (fetched 2026-09-20 — retention/ZDR/training commitments below).
> Legal index (2026-09-20, owner-confirmed): lists **only DPA, MCA, Privacy Policy** —
> **no AUP exists**; `typesafe.ai/legal/aup` returns 404 (still cited at MCA §2.3(l);
> confirmed dangling). Owner facts recorded 2026-09-20 (see [VERIFY-OPEN-1] updated).
> Quotes are verbatim excerpts under fair-use for compliance review.
> **Gates L2** — distillation is PROHIBITED on standard terms absent a written exception.
> **All sign-off fields blank** — human act.

## Clause inventory (what you asked to add)

| Clause | What it says (summary) | Why it matters for us |
|---|---|---|
| **§2.3(b)** | Do not use Services or **any Output** to **perform model distillation, train a model to imitate the output,** or develop a similar/competing product | **Bans L2 distillation** from Jev outputs on standard terms — see Q1 |
| **§2.3(g)** | Do not do security/vulnerability testing of the Services: do not "conduct any **security or vulnerability test** with respect to" the Services (§2.3(g) second limb), and more broadly do not bypass/avoid any access restriction or protection mechanism (§2.3(g)(1)–(2)) | **Bans pentest/vuln scanning of `api.typesafe.ai`** — our eval is of *our* policy on *our* dataset, not of TypeSafe infra; no probing of TypeSafe beyond normal API use |
| **§2.4** | Access Credentials: keys (`API key`, `username/password`) are **confidential**, do not share, do not authorize non-employees, notify on compromise; TypeSafe may Process credentials for provision of Services | **Keys are confidential, no embedding,** no sharing, no non-employee distribution — grounds BYOK (§3) |
| **§4.1** | Customer grants TypeSafe a worldwide non-exclusive right to use Input/Customer Data to provide Services; **in perpetuity** for Telemetry, fraud/abuse, and legal compliance; carve-out: TypeSafe **will not** include Customer Data in a dataset to **train model weights** without prior consent | Standard plan = perpetual telemetry license on our inputs; our data is not used for weight training without our consent (we may withhold) |
| **§4.3** | Telemetry = logs, hashes, summary stats, classifications, metrics, learnings related to use; TypeSafe may Process **without restriction**, including to improve Services | Telemetry fuels their product; informs why `local-only` / `redacted` matters |
| **§4.4** | DPA at `https://typesafe.ai/data-processing` incorporated by reference | DPA summarized below — see §4.4 |
| **§5** | Customer obligations: responsible for Input content/accuracy, must have disclosures/consents/permissions for TypeSafe to exercise its rights without violating Laws/third-party rights; responsible for End User / Customer User acts | **We must have consent to send any user data** — grounds Phase 1 consent + redact gates |
| **§12.3** | **Excluded Claims** (uncapped): failure to pay, **breach of §2.3, §2.4, §5**, and indemnity payment obligations — these are **excluded from the §12.1/12.2 liability caps** | Breach of the distillation ban (§2.3(b)), key-sharing (§2.4), or customer obligations (§5) is **uncapped** |
| **§13.2** | Customer indemnifies TypeSafe for claims relating to Input, Customer Applications, breach of §2.3/§2.4/§5, and End User claims — **uncapped** per §12.3 | Our Inputs (e.g., attacker-controlled commands we forward) can trigger uncapped indemnity — reinforces redact + consent + allow-list |
| **§13.1** | TypeSafe indemnifies us for IP infringement of the Services as delivered (patent/copyright/trademark/trade secret, U.S.) | Limited to their IP, not our Input pipeline |
| **§16.4** | **Publicity / branding:** neither party may use the other's name/brand/logo or announce the agreement **without prior consent**, except TypeSafe may list Customer as a customer on its site/materials (customer may request cessation) | **No "powered by Jev" / logo / announcement without consent** — see §5 flag |

AUP location: **`https://typesafe.ai/legal/aup`** (cited at MCA §2.3(l)) — **does not exist**
<!-- code-span, not a hyperlink: the URL 404s by design ([VERIFY-OPEN-1]);
     markdown-link-check skips code spans, so the gate stays green while the
     fact + recheck date below stay recorded. -->
(2026-09-20, owner-confirmed: legal index lists only DPA, MCA, Privacy Policy;
`typesafe.ai/legal/aup` 404 https+http, re-fetched for this update) — see
[VERIFY-OPEN-1] (written confirmation requested, **not a blocker** per owner).

DPA location: **https://typesafe.ai/data-processing** (Apr 24, 2026), incorporated via §4.4 — summarized below.
Privacy Policy location: **https://typesafe.ai/legal/privacy-policy** (Nov 19, 2025) —
commitments on training/disclosure/retention/hosting (see below + §4.4 summary).

Subprocessors (owner-confirmed 2026-09-20 from `https://trust.typesafe.ai/subprocessors`):
**AWS** (stores live-request data), **Modal / Nebius / CoreWeave** (process, do **not**
store), **Slack + Google Workspace** (support) — **all USA**. Hosted in the US
(Privacy Policy "International Visitors"); retention is "as long as reasonably
necessary" with no fixed SLA; ZDR is enterprise-only via `privacy@typesafe.ai`.

## Quoted sections + verdicts

### Q1 — MCA §2.3(b) (License Restrictions — distillation)

> "Customer will not do (and will not attempt to do)… any of the following:
> … (b) use the Services or any Output (defined below) to perform model
> distillation, train a model to imitate the output of the Services, or develop
> (or to facilitate the development of) a similar or competing product or service"

Verdict: **PROHIBITED unless TypeSafe grants a written exception** — training the
L2 local classifier on Jev outputs (master plan §4.1: L2 "trained from Jev outputs")
is model distillation as defined here and is banned under the standard MCA. This
verdict is cited, not ambiguous: the clause names distillation explicitly. Redesign
in §3 (L2 human-only) applies.

### Q1b — MCA §2.3(g) (Security / vulnerability testing)

> "Customer will not do… (g) bypass, avoid, remove, deactivate, or otherwise
> circumvent: (1) any access restrictions; or (2) any other software protection
> mechanisms in the Services, including any such mechanism used to restrict or
> control the functionality of any of the foregoing, **or conduct any security or
> vulnerability test with respect to any of the foregoing**;"

Verdict: **PROHIBITED** — do not conduct security or vulnerability testing of the
TypeSafe Services (incl. `api.typesafe.ai`). Our eval is of our **own** policy on
**our** dataset and our harness — not testing TypeSafe infrastructure. Normal API use
(probe ≤5, measure 3×N) is licensed use under §2.1, not vulnerability testing.
Pen-testing TypeSafe requires separate written permission.

### Q2 — MCA §2.4 (Access Credentials — confidentiality)

> "Customer and its personnel may only access the Services through the mechanisms
> designated by TypeSafe, including an API key … Customer will ensure that each
> Customer User keeps the Access Credentials **confidential and does not share them**
> with anyone else. … Customer is responsible for all actions taken in connection
> with … Access Credentials … Customer will promptly notify TypeSafe if it becomes
> aware of any compromise …"

Verdict: **Keys are confidential, no sharing, no embedding.** This grounds the
BYOK default (see §4): no embedded key in distributed binaries, no sharing with
non-employees, notify on compromise.

### Q3 — MCA §4.1 (Use of Customer Data) + §4.3 (Telemetry) + §4.4 (DPA)

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
> §4.4: "The terms of the Data Processing Agreement currently available at
> https://typesafe.ai/data-processing are incorporated herein by reference."

Verdict: **CONDITIONAL** — standard-plan traffic leaves the machine under a
perpetual telemetry license (hashes, summary statistics, "learnings related to
Customer's use"). Allowed ONLY with: (a) pre-send redaction (no secrets/source
beyond what the user approved), (b) `redacted` default / `local-only` mode per
UX checklist, (c) ZDR enterprise tier pursued for team traffic. Weight-training
on our data requires our prior consent (allowed to withhold). DPA adds
processor duties — see DPA summary below.

### Q4 — MCA §5 (Customer Obligations)

> "Customer is responsible for Input, including its content and accuracy, and will
> comply with Laws when using the Services. Customer represents, warrants, and
> covenants that it has made all disclosures, has provided all notices, and has
> obtained (and will maintain) all rights, consents, and permissions necessary
> for TypeSafe to exercise the rights granted to it in this Agreement (including
> the rights granted with respect to Input) without violating or infringing Laws
> or third-party rights."

Verdict: **We must have consent to send any user data.** This is the contractual
basis for Phase 1 consent + redact gates — no real user command goes to Jev
without disclosures/notices/rights. Reinforces `docs/redact-consent-readiness.md`
first-task gating.

### Q5 — MCA §2.3(b)/(g) + §2.4 + §5 → §§12.3, 13.2 (uncapped indemnity)

> §12.3: "**EXCLUDED CLAIMS** MEANS: (A) …; (B) CUSTOMER'S BREACH OF
> SECTION 2.3 (LICENSE RESTRICTIONS), SECTION 2.4 (ACCESS CREDENTIALS;
> CUSTOMER USERS) OR SECTION 5 (CUSTOMER OBLIGATIONS); OR (C) A PARTY'S
> PAYMENT OBLIGATIONS UNDER THE INDEMNITY SET FORTH IN SECTION 13…"
> §13.2: "Customer will defend TypeSafe from and against any third-party claim
> to the extent (a) relating to Input, (b) relating to Customer Applications …
> (c) arising out of or resulting from facts … that would result in Customer's
> breach of Section 2.3, Section 2.4, or Section 5, or (d) brought by an End User
> and related to the subject matter of this Agreement, and … will indemnify …"

Verdict: **Uncapped.** Breach of the distillation ban (§2.3(b)), bypass/security-testing
(§2.3(g)), key-sharing (§2.4), or customer obligations (§5) is **excluded from the
§12.1/12.2 liability caps** and triggers **uncapped indemnity for Input and End User
claims** (§13.2(a),(d)). Upstream risk if we forward attacker-controlled inputs.

### Q6 — MCA §16.4 (Publicity / branding)

> "Nothing in this Agreement grants either Party the right to use the name, brand,
> or logo of the other Party, and neither Party may publicly announce that the
> Parties have entered into the Agreement, except with the other Party's prior
> consent or as required by Laws; provided, however, that TypeSafe may use the
> name, brand, or logo of Customer (or Customer's parent company) for the purpose
> of identifying Customer as a licensee or customer on TypeSafe's website or in
> other promotional materials, or as part of a list of TypeSafe's customers in a
> press release or other public relations materials announcing Customer's use of
> the Services. TypeSafe will cease further use of such assets at Customer's
> written request."

Verdict: **No "powered by Jev" / logo / announcement without consent.** Do not
publish "powered by Jev," TypeSafe logo, or "we use Jev" announcements without
**prior written consent**. TypeSafe may list us as a customer unless we request
cessation. See §5 flag for wording fixes.

### Q7 — Vendor cookbook vs MCA (ambiguity → ticket, unchanged)

The official AutoResearch cookbook trains a downstream classical (CatBoost) model on
Jev probabilities (https://docs.typesafe.ai/cookbooks/autoresearch_feature_discovery,
verified 2026-09-20), which reads in tension with §2.3(b) Q1 above.
Verdict: **Still AMBIGUOUS vs PROHIBITED** — ticket to TypeSafe support/sales:
"Does §2.3(b) prohibit training a small local safety classifier on Jev outputs for
use inside our own product, and is an enterprise amendment available?"
Ticket ID: TBD (file before any L2 training; record ID here).
Until answered in writing: the PROHIBITED verdict (Q1) **stands**; the redesign in §3
(human-only training) is the controlling plan.

### Verdict summary

| Use | Verdict | Basis |
|---|---|---|
| L3 runtime judgments (allow/ask/deny) | allowed | MCA §2.1 license + §4.2 |
| Threshold tuning on OUR labels | allowed | §4.2; no Jev-output training |
| L2 classifier trained on Jev outputs | **PROHIBITED (standard MCA) unless written exception** | §2.3(b) Q1 above |
| L2 trained on human labels / deterministic rules / user confirmations **only** | allowed (redesigned) | §2.3(b) does not cover human-origin labels — see §3 |
| L2 enterprise exception | ambiguous → ticket TBD | cookbook vs §2.3(b) tension — §2.4/§16.4 enterprise path |
| Security / vuln testing of TypeSafe Services | **prohibited** | §2.3(g) Q1b above |
| API keys shared / embedded in binaries | **prohibited** | §2.4 Q2 above |
| Sending redacted states to Jev | conditional (redact + consent + ZDR path) | §4.1/§4.3 + §5 Q3 |
| Customer Input / End User indemnity | **uncapped** if it breaches §2.3/§2.4/§5 | §12.3 Excluded Claims + §13.2 |
| "Powered by Jev" / logo / announcement | **needs consent** | §16.4 Q6 above |

## DPA summary (https://typesafe.ai/data-processing — Apr 24, 2026, fetched 2026-09-20)

> Incorporated into the MCA via §4.4. Summary here is informational; the DPA itself controls.
> **Owner facts added 2026-09-20** for subprocessors/retention/hosting (see below).

- **Roles (§1.1):** Customer = controller/business, TypeSafe = processor/service provider for Customer Personal Data.
- **Scope of processing (§2.1):** only to provide Services per Documented Instructions (DPA + MCA + written instructions); must inform Customer if a legal obligation requires deviation.
- **Restrictions (§2.2):** will not sell/share, retain/use/disclose outside Documented Instructions or the direct business relationship, nor combine Customer Personal Data with third-party data (except as Data Protection Law permits).
- **Subprocessors (§3):** general authorization; list at **https://trust.typesafe.ai/subprocessors** — **owner-confirmed 2026-09-20:** **AWS** (stores live-request data), **Modal / Nebius / CoreWeave** (process, do **not** store), **Slack + Google Workspace** (support) — **all USA**; contractual protections must be substantially as protective; Typesafe remains responsible; **15-day notice + objection window** for new subprocessors (§3.2).
- **Assistance (§4):** forward data-subject requests, reasonable assistance for impact assessments / regulatory consultations (may charge reasonable fee, §4.2).
- **Security (§5.1):** reasonable technical/organizational measures; may update if not materially decreasing security.
- **Security incident (§5.2):** notify **within 72 hours** of awareness, plus investigation/mitigation assistance.
- **Audits (§5.3):** Customer may audit **once per 12 months**, at Customer cost, confidential auditor, normal business hours, minimal disruption, mutually agreed scope — may use results only for regulatory/compliance confirmation.
- **International transfers (§6):** authorized with **EU SCCs Module 2/3** (controller→processor, processor→subprocessor) + **UK Addendum**; governing law Ireland (EU SCCs Clause 17 option 1), courts Dublin; Switzerland → Swiss courts / FDPIC. Frequency: **continuous**. **Hosting is in the US** (Privacy Policy "International Visitors," Nov 19, 2025 — see below); all subprocessors USA per owner.
- **Duration (Schedule I §8):** retain **as long as necessary** for the purpose, in compliance with statute-of-limitations and Data Protection Law — no fixed retention period. **Privacy Policy "Retention" (Nov 19, 2025, owner-confirmed) restates: "for as long as reasonably necessary" — no fixed SLA.** Deletion on request is best-efforts ("take measures to delete … when no longer reasonably necessary"), unless law requires longer.
- **Data categories (Schedule I):** "Customer Personal Data, the content of which is determined and controlled by Customer" (i.e., what we send), including sensitive data if we send it; categories are customer-controlled.
- **Privacy Policy training commitment (Nov 19, 2025, owner-confirmed):** TypeSafe **will not train or fine-tune any AI/ML models on Input (your prompts / other Input)** and **will not disclose any Input to a third party other than our service providers** (see Privacy Policy "Services" + "How We Disclose"). This sits alongside MCA §4.1/§4.3 (no weight training without prior consent + perpetual telemetry license on the *other* telemetry).
- **ZDR:** **enterprise-only via `privacy@typesafe.ai`** (Privacy Policy + MCA; owner-confirmed) — standard plans have no ZDR.
- **Not in the DPA (still [VERIFY-OPEN]):** fixed retention days, deletion SLA after termination, subprocessors list currency beyond today's snapshot — see [VERIFY-OPEN-2]; keep the audit question open.

## L2 redesign (human-only)

L2 is **PROHIBITED** from Jev-distilled training on standard terms (§2.3(b) Q1).
Redesign: **train only on human labels, deterministic rule outputs, and user
confirmations** — never on Jev outputs (see §3). Ensure no dataset label or
training feature is derived from Jev outputs (see §3 checklist). L2 ships only
if it meets the same eval gate as Jev on its share (master plan §4.1 caveat).

## Access mode (BYOK — only real-data mode, Jev off by default)

**Bring your own Jev API key (BYOK) is the only real-data mode.** No embedded
key in distributed binaries (§2.4). **Jev is off by default** — `local-only`
(default) never sends; `redacted` (BYOK, user-supplied key, explicit opt-in
consent) is the only path that sends real user commands to Jev. Proxy mode
through our servers stays **blocked** until legal review (see §4 and ADR-0004).

Consent must disclose that commands go to **TypeSafe's US infrastructure** with
**unspecified retention** ("as long as reasonably necessary") and that
**non-US users' data is transferred to the US** (Privacy Policy "International
Visitors" + subprocessors all USA) — see `docs/privacy-dataflow.md` + `docs/redact-consent-readiness.md`.

## [VERIFY-OPEN] items

- [VERIFY-OPEN-1] AUP — **does not exist** (owner-confirmed 2026-09-20: legal index
  lists **only DPA, MCA, Privacy Policy**; `typesafe.ai/legal/aup` 404 https+http
  re-fetched for this update; still cited at MCA §2.3(l) as dangling ref) — **not a
  blocker** per owner. Action: request **written confirmation from TypeSafe that no
  AUP exists / that eval traffic is not subject to an unpublished AUP**, record
  the email/ticket ID + date here. Prior hunt: `https://typesafe.ai/legal/aup`
  (404), `https://docs.typesafe.ai/legal` (DPA/MCA/Privacy Policy only),
  `https://docs.typesafe.ai/llms.txt` (no AUP), MCA footer (Terms/Privacy only),
  `https://trust.typesafe.ai/` (no AUP) — 2026-09-20. Owner: product/legal.
  Recheck: written confirmation, not a URL search.
- [VERIFY-OPEN-2] Confirm DPA open items: fixed retention/deletion SLA beyond
  "as long as reasonably necessary" (DPA Schedule I §8 + Privacy Policy Retention;
  no fixed SLA), subprocessors list currency beyond 2026-09-20 snapshot
  (AWS / Modal / Nebius / CoreWeave / Slack / Google Workspace — all USA),
  Telemetry opt-out / ZDR need on standard plans (ZDR enterprise-only via
  `privacy@typesafe.ai` per Privacy Policy). Owner: product/legal. Recheck:
  2026-10-20.
- [VERIFY-OPEN-3] Distillation exception ticket (see Q7) — file + record ID.
  Owner: product. Recheck: 2026-10-20.

## Human sign-off

- [ ] Reviewer (CODEOWNERS, legal/product): __________ Date: __________
- [ ] L2 redesign (human-only training) accepted: __________
- [ ] BYOK default + embedded-key prohibition accepted: __________
- [ ] Branding-consent rule (§16.4) acknowledged: __________
- Gate effect: L2 work (Phase 3) may not consume Jev outputs until this sign-off
  plus either ticket clearance or the human-only redesign ADR update exists.
