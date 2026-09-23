# Status — algorithco guard (living document, updated in place)

> Last updated: 2026-09-23, previous snapshot: `docs/STATUS-2026-09-20.md` (point-in-time audit, kept for the trail — DEFERRED.md §6.4 and ADR-0009 pin it by name; do not delete it).
> Older snapshots live only in git history — do not add new `STATUS-*.md` files going forward; update this file instead.
> This file asserts no gate pass and fills no sign-off. Numbers below are pointers to their canonical sources, not copies.

## Where to look (authoritative sources)

- Gate criteria + thresholds → [`docs/exit-gate-P0.md`](./exit-gate-P0.md) (canonical; §1.1/§1.3/§1.5).
- Skipped P0 items + milestone mapping → [`docs/DEFERRED.md`](./DEFERRED.md) (live tracker under waiver ADR-0009).
- Security-critical review queue → [`docs/SECURITY-REVIEW-QUEUE.md`](./SECURITY-REVIEW-QUEUE.md).
- Decisions D-01..D-10 + D-A..D-G → [`docs/decision-log.md`](./decision-log.md) + [`docs/adr/`](./adr/README.md).
- TypeSafe/Jev evidence (ToS, API, claims, blockers research log) → [`docs/verify/`](./verify/blocked-on-typesafe.md) + [`docs/DEFERRED.md`](./DEFERRED.md) §4 (live).
- Build order + per-phase exit gates → [`../plans/00-index-build-order.md`](../plans/00-index-build-order.md).

## Current position (pointers, 2026-09-23)

- Branch `main` at `1626f4a` (PR #1 merged: P1-08 shadow/enforce CLI/audit, redact crate, CEL-vs-DSL spike + ADR-0012 draft, GitHub gating). CI was green on the merge head; merge had 0 reviews — post-merge CODEOWNERS review + branch protection are follow-ups (see merge notes).
- P0 exit gate: **DRAFT, unchecked, unsigned** — see `exit-gate-P0.md:5` + §5 sign-off (blank). Waiver ADR-0009 governs private-MVP work; gate is deferred, not cancelled.
- Jev: **shadow-only** per ADR-0008 (draft); L2 distillation **PROHIBITED** on standard MCA §2.3(b) per `verify/jev-tos.md` Q1; no real user data to Jev until redact + `--show-egress` + `local-only` default + consent are shipped + human-reviewed (DEFERRED §3).

## Update protocol

1. Edit this file in place (date + `main` commit on the top line).
2. Never copy threshold numbers here — link to `exit-gate-P0.md` sections.
3. Never check a box or fill a `__________` here — sign-offs live in the gate/ADR files.
4. Dated snapshots: tag `main` if a point-in-time record is needed; do not create `STATUS-<date>.md` files.
