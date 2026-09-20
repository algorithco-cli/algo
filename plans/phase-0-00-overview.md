# Phase 0 Overview — Contracts and Evidence

> No product code. Produces `proto` v0 contracts, `eval` evidence (200-300 actions + harness + Jev numbers), `docs` governance. Exit gate per §8.

## Goal / non-goals

**Goal:** answer "is Jev accurate/fast enough, and what are the contracts?" before building.
**Non-goals:** `core` crates, `agent` daemon/adapters, `backend`, `dashboard`, L2 training, scanner/verifier logic. Only throwaway redacted probe client in `eval/jev_client/`.

## File map (01–05)

- `phase-0-01-docs-adr-threat-model.md`
- `phase-0-02-proto-contracts-v0.md`
- `phase-0-03-eval-dataset-harness.md`
- `phase-0-04-jev-access-measurement.md`
- `phase-0-05-exit-gate.md`

## Task board

- `P0-DOCS-1` skeleton, `P0-DOCS-2` ADR process, `P0-DOCS-3` threat v0, `P0-DOCS-4` license+name ADRs, `P0-DOCS-5` tracking issues
- `P0-PROTO-1` buf+CI, `P0-PROTO-2` events, `P0-PROTO-3` decision, `P0-PROTO-4` dataset, `P0-PROTO-5` wire stubs alpha, `P0-PROTO-6` codegen+tag
- `P0-EVAL-1` skeleton, `P0-EVAL-2` labeling guide, `P0-EVAL-3` dataset 200-300, `P0-EVAL-4` harness, `P0-EVAL-5` baselines+CI, `P0-EVAL-6` question variants
- `P0-JEV-1` dossier, `P0-JEV-2` ToS, `P0-JEV-3` probe client, `P0-JEV-4` full measurement, `P0-JEV-5` = EVAL-6 joint
- `P0-GATE-1` thresholds+matrix, `P0-GATE-2` readiness

## Dependency graph + build order (Waves 0–4)

```
Wave0 (day1, parallel): P0-DOCS-4 ─┬─► P0-DOCS-1
                        P0-JEV-1 ───┘
Wave1: P0-DOCS-2, P0-PROTO-1 ─► P0-PROTO-2/3 ─► P0-PROTO-4 ─► P0-PROTO-5(stub)
Wave2: P0-EVAL-1 ─► P0-EVAL-2 ─► P0-DOCS-3
Wave3: P0-EVAL-3 ─► P0-EVAL-4 ─► P0-EVAL-5 | P0-JEV-2, P0-JEV-3
Wave4: P0-PROTO-6, P0-EVAL-6, P0-JEV-4 ─► P0-GATE-1 ─► P0-GATE-2
```

Critical path: labeling quality → harness → Jev measurement → gate.

## Owners / tracking

One `docs` issue per workstream, linking `proto`→`eval` PRs. `infra` out-of-scope. Human sign-off required: threat model, ToS verdict, gate thresholds.

## Status

- [ ] proto v0 tagged
- [ ] dataset v0.1 tagged
- [ ] harness + baselines green
- [ ] Jev multi-region report published
- [ ] gate signed, Phase 1 unblocked
