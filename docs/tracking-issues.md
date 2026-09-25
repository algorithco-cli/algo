# Tracking issues — Phase 0

One issue per workstream. Every P0 PR links its issue.

| Issue | Workstream | Task IDs | Exit check |
|-------|------------|----------|------------|
| #1 docs skeleton | docs | P0-DOCS-1 | fresh-agent finds plan + ADR dir < 2 min |
| #2 ADR process | docs | P0-DOCS-2 | template merged, D-A..D-F drafts with owners/dates, links green |
| #3 threat-model v0 | docs | P0-DOCS-3 | each boundary has threat + mitigation; human sign-off |
| #4 license + naming | docs | P0-DOCS-4 | ADR 0002 merged; blocks proto tag |
| #5 tracking issues | docs | P0-DOCS-5 | all P0 PRs link an issue |
| #6 proto workspace | proto | P0-PROTO-1 | `buf lint` green, workspace set |
| #7 proto contracts v0 | proto | P0-PROTO-2 | events/decision/dataset schema tagged |
| #8 eval dataset | eval | P0-EVAL-1 | 200-300 labeled actions, redacted |
| #9 eval harness | eval | P0-EVAL-2 | harness + baseline metrics |
| #10 Jev access | jev | P0-JEV-1 | API/SDK dossier + ToS note |
| #11 Jev measurement | jev | P0-JEV-2 | false-allow + p50/p99 latency measured |
| #12 exit gate | gate | P0-GATE-1 | threshold matrix + Phase-1 go/no-go |

Conventions: `P0-DOCS-*`, `P0-PROTO-*`, `P0-EVAL-*`, `P0-JEV-*`, `P0-GATE-*`.
Cross-repo order: docs issue -> proto PR -> version bump -> consumer PRs.
