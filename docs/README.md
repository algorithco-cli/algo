# algorithco guard — Docs index (which file is authoritative for what)

> Fresh-agent path: this file → [adr/](./adr/README.md) → `../plans/00-index-build-order.md`.
> Scope is governed by [`AGENTS.md`](./AGENTS.md) + waiver [`adr/0009-p0-gate-waiver.md`](./adr/0009-p0-gate-waiver.md) — not by any scope sentence that used to live here.

## Compliance trackers (do not weaken; sign-offs are human acts)

- [DEFERRED.md](./DEFERRED.md) — **live tracker** of every skipped P0 item + the milestone it must be done BEFORE. Status changes go here.
- [SECURITY-REVIEW-QUEUE.md](./SECURITY-REVIEW-QUEUE.md) — security-critical files written under the waiver, queued for human review.
- [exit-gate-P0.md](./exit-gate-P0.md) — **canonical** gate criteria + thresholds (§1.1/§1.3/§1.5); DRAFT, unchecked, unsigned.
- [STATUS.md](./STATUS.md) — living status (updated in place, pointer-only, no copied numbers).
- [STATUS-2026-09-20.md](./STATUS-2026-09-20.md) — point-in-time audit snapshot; kept because DEFERRED §6.4 + ADR-0009 pin it by name.

## Decisions

- [decision-log.md](./decision-log.md) — D-01..D-10 + D-A..D-G table (no threshold numbers; links to ADRs).
- [adr/](./adr/README.md) — Architecture Decision Records (Status/Context/Decision/Alternatives/Consequences/Verification + Sign-off).
- [split-plan-draft.md](./split-plan-draft.md) — **superseded** monorepo-split draft (do not execute; waiver keeps one private monorepo).

## Security / privacy / safety evidence

- [threat-model-v0.md](./threat-model-v0.md) — assets, boundaries, abuse cases (sign-off blank).
- [security-model.md](./security-model.md) — fail-safe, rules outrank models.
- [privacy-dataflow.md](./privacy-dataflow.md) — local-only / redacted / full + consent drafts (spec, unshipped parts noted inside).
- [redact-consent-readiness.md](./redact-consent-readiness.md) — redact + egress + consent readiness; first Phase 1 task gate.
- [redact-crate-design.md](./redact-crate-design.md) — `core/crates/redact` design (Draft, unreviewed).
- [public-exposure-review.md](./public-exposure-review.md) — 8 items to keep non-public before license/legal (Draft proposal).
- [verify/](./verify/blocked-on-typesafe.md) — Jev evidence dossiers: ToS (`jev-tos.md`), API/SDK (`jev-api.md`), claim ledger (`jev-claims.md`), TypeSafe blockers research log (`blocked-on-typesafe.md`, dated — live status in DEFERRED §4), measurement runbook (`MEASUREMENT-CHECKLIST.md`), wording flags (`distillation-wording-flags.md`, fixes pending).

## Process / planning pointers

- [AGENTS.md](./AGENTS.md) — how to work in `docs/` (commands, gates, conventions).
- [CONTRIBUTING.md](./CONTRIBUTING.md) — commits, PRs.
- [CODEOWNERS](./CODEOWNERS) — human review paths (agent approval never sufficient).
- [roadmap.md](./roadmap.md) — Phase 0–4 build order (plan files in `../plans/` are executable).
- [tracking-issues.md](./tracking-issues.md) — one issue per workstream.
- [github-org-plan.md](./github-org-plan.md) — org/repo/PR-gating plan (branch protection applied separately, human admin act).
- [branch-protection.json](./branch-protection.json) — required-checks doctrine for the admin apply scripts.
- [p1-exit-gate-report.md](./p1-exit-gate-report.md) — Phase 1 exit-gate report.
- [design-tokens-verify.md](./design-tokens-verify.md) — Variant 1 token verification checklist (single source: `../plans/design-tokens.md`).

## Source plans (outside this folder; build source of truth)

- `../plans/00-index-build-order.md` — canonical layout, DAG, build order, per-phase gates.
- `../jev-guard-project-plan.md` — master plan (historical source; §3 carries a canonical pointer to the index).
- `../plans/phase-0-01-docs-adr-threat-model.md`, `../plans/90-crosscutting-gates-ux-decisions.md` — docs/ADR process + gates/UX/decisions source sections.
