# ADR-0009: P0 gate waiver for a private MVP (owner-instructed)

## Status

draft — 2026-09-21 (owner-instructed per `docs/STATUS-2026-09-20.md` owner feedback)
**Countersignature: __________ Date: __________ (leave blank — human act)**
This is a **waiver, not a pass** — it defers P0 exit-gate sign-offs and documentation debt to allow a private, unreleased MVP. It does **not** mark any gate as passed.

## Context

P0 exit gate (`docs/exit-gate-P0.md:5` + `AGENTS.md:44`) requires every `§3a` box checked + human gate sign-off before any Phase 1 product code. As of `642b4e6` (`docs/STATUS-2026-09-20.md`), the gate is **DRAFT, unchecked, unsigned** (all `§3a` boxes unchecked, all sign-off fields `__________`, threat model `docs/threat-model-v0.md:58-59` blank, `docs/adr/0001-license.md:5` draft, etc.). EVAL-6 is time-boxed 2–3 weeks (`eval/questions/EVAL-6-STARTED.md:134-160`), dataset is 100% synthetic (240/0), human review (101 blinded) not yet done, redact crate not yet built, and TypeSafe confirmations (retention, regions, written AUP) are pending.

Owner decision 2026-09-21: **SKIP the P0 exit-gate sign-offs and the documentation debt for now and continue from the next step of the plans** (`plans/00-index-build-order.md` → Phase 1). The gate is **deferred, not cancelled** — every skipped item must be done before the milestone that needs it (see `docs/DEFERRED.md`).

The repo `algorithcoguard/algorithco-guard` is **public** today (`gh api ... --jq .visibility` → `public` on 2026-09-20) but the MVP is **private until release** (owner decision, final).

## Decision

**Allow product code to be written now, in the existing monorepo, for an unreleased, PRIVATE MVP — without marking any P0 gate as passed.**

- The one-repo layout (`proto`, `core`, `agent`, `backend`, `dashboard`, `web`, `eval`, `docs` as top-level packages in `algorithcoguard/algorithco-guard`) is **final** (owner decision). Do not split into `guard-*` repos and do not create other repos. Keep packages independent via proto contracts.
- Keep the naming already **accepted in ADR-0002 unchanged**: CLI `algo` (`algo init|doctor|status|why|log|pause`), crates `algo-*`, home `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db` (owner decision, final). ADR-0010 naming amendment is **superseded / do not apply**.
- The repo **stays PRIVATE until release** (owner decision). Before release, every item in `docs/DEFERRED.md` and every file in `docs/SECURITY-REVIEW-QUEUE.md` must be human-reviewed.
- This waiver does **not** mark any `docs/exit-gate-P0.md:5` box as checked and does **not** fill any `__________` sign-off field. Those stay blank.
- Rules that stay on (prevent irreversible harm) are enumerated in `docs/DEFERRED.md:3` and `AGENTS.md:0.5-0.10` (fail-safe `ASK`, Jev OFF with only mock, no secrets, backup/restore, latency budgets, redact first) — they remain merge-blockers.

## Alternatives

- **Keep the gate closed until every sign-off is done**: rejected by owner — it would block the private MVP by 2–3 weeks (EVAL-6) + human review + legal, while a private MVP can safely proceed with the staying rules + `redact` first + Jev off.
- **Mark the gate as passed now**: rejected — it would misrepresent the evidence (synthetic 240, n=72/72, p50 FAIL 399/465 vs 250, no human κ) as a public-release readiness, which it is not.

## Consequences

### Positive

- Phase 1 `core`/`agent` critical path can start immediately (redact first, then types/fingerprint/shell/policy/provider mock, daemon/hook/adapter, audit/CLI shadow) without waiting for the 2–3-week EVAL-6 box or the second human grader.
- `docs/DEFERRED.md` + `docs/SECURITY-REVIEW-QUEUE.md` keep the debt visible and milestone-gated, so the waiver is auditable.

### Negative

- The repo carries deferred debt: it is **not** releasable, not demonstrably safe on real-world data, and not legally cleared for public. Any `docs/exit-gate-P0.md` quote must include the `Scope caveat` (synthetic distribution, Phase 1 shadow is real-world validation).
- Risk that deferred items are forgotten — mitigated by `docs/DEFERRED.md` being a merge-blocker before the milestone that needs each item (public release / real user data / Jev enabled / proxy mode).

## Verification

- `AGENTS.md:37-45` (Phase discipline) is updated to allow product code in the PRIVATE monorepo for the unreleased MVP per this waiver (see `AGENTS.md:1` edit, countersignature blank).
- `docs/DEFERRED.md` lists every skipped sign-off/human review/ratification/dataset expansion/EVAL-6/license/TypeSafe confirmation with the milestone it must be done **BEFORE**.
- `docs/SECURITY-REVIEW-QUEUE.md` lists every security-critical file written under this waiver for later human review.
- Both files are updated at each milestone (report after each milestone per owner).
- Repo stays `PRIVATE` until the deferred public-release items are signed (verified via `gh api repos/algorithcoguard/algorithco-guard --jq .visibility`).

## Sign-off (leave blank — human act)

- [ ] Owner direction acknowledged (waiver, not a pass): __________ Date: __________
- [ ] Countersignature (second human, confirms waiver scope): __________ Date: __________
- Gate effect: this waiver **does not** check any `docs/exit-gate-P0.md:5` box and **does not** fill any `__________` in that file, `docs/adr/*`, `docs/threat-model-v0.md`, `eval/datasets/v0.1/DATASET.md`, `eval/datasets/v0.1/REVIEW-PACKET.md`, or `docs/redact-consent-readiness.md`. Those stay blank until their milestone.

## References

- Owner feedback 2026-09-21: "SKIP the P0 exit-gate sign-offs and the documentation debt for now and continue … The gate is deferred, not cancelled."
- `docs/STATUS-2026-09-20.md` (read-only audit) — evidence for the deferred items.
- `plans/00-index-build-order.md:54-65` — build order `proto → core → agent/backend → dashboard` (Phase 1 critical path first).
