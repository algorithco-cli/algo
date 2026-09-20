# Flagged "distillation" / "powered by Jev" wordings — Draft fixes (2026-09-20)

> **Status:** draft — 2026-09-20. No decision made; human review required.
> Tracks every place that assumes distillation of Jev outputs or "powered by Jev"
> branding, with a proposed neutral wording per MCA §2.3(b) (PROHIBITED without
> written exception) and §16.4 (needs consent). All sign-off fields blank —
> human act. The plan/ADRs below are **flagged, not edited in place** — fixes
> below are proposals for their owners.

## A. Distillation assumptions (MCA §2.3(b) — prohibited without exception)

| # | File | Current wording | Issue | Proposed fix (Draft) | Owner |
|---|---|---|---|---|---|
| A1 | `jev-guard-project-plan.md:58` | `` `eval` — … threshold tuning, **distillation training**, model export `` | Describes `eval` as distillation training from Jev | `eval — … threshold tuning, **human-label/consent-only training** (Jev-distilled only with a written TypeSafe exception per MCA §2.3(b)), model export` | @algorithcoguard/eval |
| A2 | `jev-guard-project-plan.md:139-143` | "**Caveats on L2 distillation** … `[VERIFY]` TypeSafe's terms: are we allowed to use Jev outputs to train another model?" | Flagged as [VERIFY]; now answered: PROHIBITED | Keep caveat but replace `[VERIFY]` with "`PROHIBITED on standard MCA §2.3(b) unless written exception — see `docs/verify/jev-tos.md:Q1` + `plans/phase-3-l2-model.md:1` human-only redesign`"; file ticket per Q7 | @algorithcoguard/eval+legal |
| A3 | `jev-guard-project-plan.md:362` | `Terms of use for training on Jev outputs [VERIFY]` | Now answered | Change to "`Terms of use … PROHIBITED on standard MCA §2.3(b) (see `jev-tos.md:Q1`); redesign is human-only (`phase-3-l2-model.md` Draft) unless exception`" | @algorithcoguard/eval |
| A4 | `README.md:39` | `L2 — Small local classifier (CPU, **distilled**)` | Implies Jev-distilled | `L2 — Small local classifier (CPU, **human-label trained; distilled only with a written TypeSafe exception — see MCA §2.3(b)**)` | @algorithcoguard/eval |
| A5 | `README.md:55` | `` `eval` — … threshold tuning, **distillation** `` | Same as A1 | Same fix as A1 | @algorithcoguard/eval |
| A6 | `docs/github-org-plan.md:17` | `` `eval/` — … thresholds, **distillation** `` | Same | `eval/ — … thresholds, **human-only training (Jev-distilled only with exception)**` | @algorithcoguard/eval |
| A7 | `plans/phase-3-l2-model.md` | Title "**L2 Local Model (Distillation, Gated)**" and body "unless TypeSafe grants a written exception" — **already fixed 2026-09-20** to `Human-only training, Gated [DRAFT]` + human-only checklist at `phase-3-l2-model.md:1` | Was distillation-assuming; now correctly PROHIBITED draft | No further fix — this row records that the plan file **is already the Draft fix**; keep as Draft until exception or human-only artifact ships | @algorithcoguard/eval |
| A8 | `docs/verify/jev-tos.md:Q1`, `docs/verify/blocked-on-typesafe.md:A1/R3` | Correctly state PROHIBITED + human-only redesign | Correct | No fix — reference target for A1–A6 | — |

**Global rule (applies everywhere):** no dataset `label`/`rationale` or training feature column may be derived from a Jev output (`jev_label`/`jev_confidence`/`jev_probabilities`) without a superseding distilled ADR that records the TypeSafe exception ID. This rule is enforced by `plans/phase-3-l2-model.md:1` checklist and `docs/verify/jev-tos.md:Q1` gate.

## B. "Powered by Jev" / branding assumptions (MCA §16.4 — needs consent)

| # | File | Current / risk wording | Proposed fix (Draft) |
|---|---|---|---|
| B1 | Any README / web / plan phrase "powered by Jev" | **Search 2026-09-20: 0 hits** for the literal `powered by Jev` / `powered by jev` (case-insensitive) — no current violation found | Keep 0 hits. Proposed neutral phrasing if branding is desired: "uses Jev **with TypeSafe written consent**" or "Jev-compatible (BYOK)" — do not publish "powered by Jev," TypeSafe logo, or "we use Jev" announcements without **prior written consent** per MCA §16.4. If TypeSafe consent is obtained, record the consent ID/date in `docs/verify/jev-tos.md:Q6`. |
| B2 | `docs/verify/jev-tos.md:Q6` + `blocked-on-typesafe.md:R5` | Correctly state needs consent — **already flagged** | No fix — these are the reference targets for B1. |

## Verification

- Re-scan: `grep -ri "distill" --include="*.md"` hits above are A1–A6 (+ this file + correctly-flagged docs); no new product assumption may be added without an owner + date.
- Re-scan: `grep -ri "powered by jev"` hits **0** (verified 2026-09-20).
- Phase 1 docs that assume Jev availability must state **BYOK default, no embedded key** per `docs/adr/0004-byok-vs-proxy.md` + §16.4.

## Sign-off (leave blank — human act)

- [ ] A1–A6 wording fixes reviewed + owners assigned: __________ Date: __________
- [ ] B1 neutral branding wording approved (or consent requested): __________ Date: __________
- [ ] `plans/phase-3-l2-model.md` Draft (human-only) remains controlling until superseded: __________ Date: __________
