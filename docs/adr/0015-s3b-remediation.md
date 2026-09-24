# ADR-0015: §3b remediation — quarantine vs scoped exception (D-I)

## Status

draft — 2026-09-24 (agent-drafted per owner direction "skip human action, continue
with our plan"; **not accepted — human merge + sign-off required before it takes effect**).
This draft decides nothing. The gate stays FAIL until a human accepts one path below.

## Context

Waiver ADR-0009 was retired 2026-09-24 (owner direction) and the P0 exit gate
(`docs/exit-gate-P0.md`) is enforced. The gate's §3b verdict is **FAIL**: product code
written 2026-09-21–24 exists in `core/`, `agent/`, `backend/`, `dashboard/`, `web/`,
and `packages/contracts/` (see gate file §3b addendum; file inventory in
`docs/SECURITY-REVIEW-QUEUE.md`, all unreviewed). Plan `plans/phase-0-05-exit-gate.md`
(P0-GATE-2) forbids Phase 1 product code before the gate passes. One remediation path
must be accepted before any further product-code work or any gate-pass claim.

## Decision

None yet (draft). The proposed decision, for human accept/reject:

> **Grant a scoped exception (path B)**: keep existing product code in-tree as
> **quarantined-by-default** — no new product-code PRs merge, no release, no real-data
> egress, Jev stays OFF — while the P0 backlog (thresholds, EVAL-6, κ re-review,
> redact/consent review, license/legal) is worked to a gate pass. Quarantine lifts
> only when `docs/exit-gate-P0.md` §3a is fully checked + §5 signed.

## Alternatives

- **Path A — remove/quarantine product code now**: `git rm` (or move to a clearly
  marked quarantine branch, never `main`) all product code until the gate passes, then
  re-land behind the gate. Cleanest gate semantics (FAIL → clean → pass), but destroys
  working, tested code (cli 11 + daemon 49 + redact 10 tests green; init→doctor→uninstall
  E2E proven) and all hardening context; re-landing risks re-introducing drift.
  Rejected in the proposal (not by a human) because the code is the hardening substrate
  the gate's own `real user data` milestone requires reviewed — deleting it also deletes
  the review target.
- **Path C — re-waive**: write a new waiver covering product code. Rejected outright:
  the owner retired the waiver model 2026-09-24 ("delete private mvp rule"); a new
  waiver would contradict that direction.

## Consequences

### Positive (if path B is accepted)

- Gate stays honest (FAIL recorded, no silent pass) while reviewable code exists to review.
- The `real user data` milestone keeps a concrete review target (redact/consent/daemon paths).

### Negative

- `main` carries known-unreviewed security-critical code; any use beyond local
  development review is unsafe. Mitigation: exception conditions (no release, no egress,
  Jev OFF) are CI-checkable where possible and human-checked otherwise.
- Path B normalizes "FAIL but proceed" — must not become precedent; this ADR is
  single-scope (§3b only, expires at gate pass).

## Verification

- Gate file keeps the FAIL verdict until §3a + §5 are complete (this ADR alone
  changes no box and fills no `__________`).
- Quarantine conditions auditable: `grep -r ALGO_JEV_API_KEY` shows env-only use;
  daemon defaults local-only (49 daemon tests); Jev feature OFF by default.
- Human acceptance = merge of this ADR to `accepted` + dated sign-off below.

## Sign-off (leave blank — human act)

- [ ] Path accepted (B, or A with removal plan): __________ Date: __________
- [ ] Exception conditions acknowledged (no release, no egress, Jev OFF): __________ Date: __________
