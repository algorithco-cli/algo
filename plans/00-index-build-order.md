# algorithco guard — Plans Index + Build Order

> Source: `../jev-guard-project-plan.md` (397 lines, master plan).
> Product: **algorithco guard**. CLI: **`algo`** (`algo init|doctor|status|why|log|enforce|pause|login|policy`).
> Naming: proto package `algorithco_guard.v0`, Rust crates `algo-*`, home dir `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db`.
> `plans/` is the executable breakdown. Each file is one reviewable scope.
> Build order is strict: `proto → core → agent/backend → dashboard`. `eval` gates thresholds.

## File map

### Index (this file)
- `00-index-build-order.md`

### Phase 0 — Contracts and evidence (no product code)
- `phase-0-00-overview.md` — goal, waves, task board
- `phase-0-01-docs-adr-threat-model.md` — docs skeleton, ADR process, threat v0
- `phase-0-02-proto-contracts-v0.md` — buf workspace, events/decision/dataset v0
- `phase-0-03-eval-dataset-harness.md` — 200-300 labels, harness, baselines
- `phase-0-04-jev-access-measurement.md` — API/SDK dossier, ToS, probe, measurement
- `phase-0-05-exit-gate.md` — thresholds, decision matrix, Phase-1 readiness

### Phase 1 — Local MVP (no cloud)
- `phase-1-00-overview-gates.md`
- `phase-1-01-adr-policy-language.md` — [DECISION] CEL vs DSL
- `phase-1-02-core-types-fingerprint.md`
- `phase-1-03-core-shell-analysis.md`
- `phase-1-04-core-policy-redact.md`
- `phase-1-05-core-provider.md`
- `phase-1-06-agent-daemon-hookclient.md`
- `phase-1-07-agent-claude-adapter-shell-only.md`
- `phase-1-08-agent-cli-audit-shadow.md`
- `phase-1-09-quality-latency-eval.md`
- `phase-1-10-exit-gate-runbook.md`

### Phase 2 — Enforcement + second capability
- `phase-2-enforcement-verifier-loop-edit-web.md`

### Phase 3 — Cloud + teams
- `phase-3-backend.md`
- `phase-3-dashboard.md`
- `phase-3-l2-model.md`

### Phase 4 — Expansion
- `phase-4-adapters.md`
- `phase-4-scanner-questions.md`
- `phase-4-tui-github-windows.md`

### Cross-cutting
- `90-crosscutting-gates-ux-decisions.md` — §6 gates, §7 UX, §9 10 items, §10 agreement
- `design-tokens.md` — Variant 1 DECIDED single source (dashboard + web + docs), consumed by P2-08 and P3 dashboard

## Global dependency DAG

```
docs-skeleton ─┬─► ADR-process ─► threat-v0
               ├─► proto-v0 (buf lint/breaking, tag) ─► core(types→fingerprint→shell→policy→redact→provider)
               │                                              │
               │                                              └─► agent(daemon+hook→claude-shell→CLI/audit+shadow)
               ├─► eval(dataset→harness→baselines) ─► jev-measure ─► exit-gate
               └─► web/docs (parallel, privacy text must match behavior)

P1 done ─► P2(proto deltas→thresholds→enforce→verifier/loop→edit/write→web)
        ─► P3(proto backend API→core verify→backend→agent sync→dashboard→L2 gated)
        ─► P4(proto findings→codex/opencode spikes→scanner→questions→TUI/github/windows)
```

No cycles. `proto` tag first, consumers pin exact version, generate at build, never hand-edit generated code.

## Task ID convention

- `P0-DOCS-*`, `P0-PROTO-*`, `P0-EVAL-*`, `P0-JEV-*`, `P0-GATE-*`
- `P1-CORE-*`, `P1-AGENT-*`, `P1-QUAL-*`
- `P2-*`, `P3-*`, `P4-*`

Each cross-repo change: `docs` tracking issue → `proto` PR → version bump → consumer PRs.

## Per-phase exit gates (hard blockers)

- **P0:** measured Jev false-allow + latency (p50 <250ms / p99 <800ms L3). If fail → redesign use case, no product code.
- **P1:** L0/L1 p50<3ms/p99<10ms, eval gate pass, macOS+Linux `algo init`/`algo uninstall` byte-identical, ≥3 users × ≥3 days shadow.
- **P2:** prompt-reduction measured, false-allow within threshold, no fail-open in review+fuzz.
- **P3:** login→org→policy→daemon→audit→dashboard E2E, backup/restore tested, L2 parity if ToS-cleared.
- **P4:** Codex/OpenCode E2E with degradation matrix, scanner precision/recall published, Choice-only proven, Windows matrix green.

## Non-negotiables (from §2, enforced everywhere)

1. Fail-safe → `ask`, never `allow` on error/timeout/crash/parse-fail.
2. Deterministic rules outrank models.
3. Latency budgets in CI — over-budget = no merge (or ADR).
4. Local-first, privacy-by-default (redact before egress, `local-only/redacted/full`), explainable (`algo why`), reversible install, provider abstraction, no invented APIs (`[VERIFY]`).
