# algorithco guard

> Intelligent control layer for CLI coding agents (Claude Code, Codex CLI, OpenCode).
> Attaches via hooks / plugins / MCP. Makes agents safer, quieter, and auditable.
> Product **DECIDED**: `algorithco guard`. CLI: **`algo`**. (canonical naming: [`docs/adr/0002-naming.md`](docs/adr/0002-naming.md))

## CLI

```
algo init | doctor | status | why | log | enforce | pause | login | policy
```

| Command | Purpose |
|---|---|
| `algo init` | Detect agents → show diff → per-agent consent → backup + additive hook install → privacy prompt → `algo doctor`. ~30s target. |
| `algo doctor` | Verify install, daemon, socket, DB, agent configs. |
| `algo status` | Counts (allowed / asked / blocked), savings, profile + threshold source. |
| `algo why` | Explain last decision: action + reason + confidence + source + latency. |
| `algo log` | Local audit log. `--show-egress` inspects exactly what would leave the machine. |
| `algo enforce [on\|off]` | Shadow → enforce switch. Explicit opt-in after shadow trust. |
| `algo pause` / `algo resume` | One-step stop / resume. Works even if daemon is broken. |
| `algo login` | OAuth device flow for team/cloud features (Phase 3). |
| `algo policy` | View / dry-run policy bundles. |

## Local paths (convention, DECIDED)

- Home: `~/.algo/`
- Socket: `~/.algo/algo.sock` (Windows: named pipe, see Phase 4)
- DB: `~/.algo/audit.db` (SQLite WAL — local audit log, cache, preferences)

## Decision pipeline (L0 → L4)

Most requests never reach Jev. Hypotheses — measured in Phase 0/1, not facts.

| Level | What | Target |
|---|---|---|
| L0 | Hard deny/allow rules on shell syntax tree | p50 < 3 ms / p99 < 10 ms |
| L1 | Cache by normalized action fingerprint | p50 < 3 ms / p99 < 10 ms |
| L2 | Small local classifier (CPU, human-labeled/deterministic training only — never Jev outputs, MCA §2.3(b)) | p50 < 10 ms / p99 < 25 ms |
| L3 | Jev evaluation (remote) | p50 < 250 ms / p99 < 800 ms |
| L4 | Ask the user (universal fallback) | human |

Fail-safe: any error / timeout / crash / parse-fail → `ask`. Never `allow`.

## Repo layout / dependency DAG (canonical: [`plans/00-index-build-order.md`](plans/00-index-build-order.md))

Monorepo `algorithcoguard/algorithco-guard` (private until release): `proto → core → agent/backend → dashboard`, `eval` gates thresholds.
Strict build order, per-package specs, and the full DAG live in the index — this section intentionally keeps only this summary + link.

## Plans (executable breakdown)

Start here: [`plans/00-index-build-order.md`](plans/00-index-build-order.md) (strict build order:
`proto → core → agent/backend → dashboard`, `eval` gates thresholds).

- Phase 0 (now, no product code): `plans/phase-0-00-overview.md` … `plans/phase-0-05-exit-gate.md`
- Phase 1 (local MVP): `plans/phase-1-00-overview-gates.md` … `plans/phase-1-10-exit-gate-runbook.md`
- Phase 2: `plans/phase-2-enforcement-verifier-loop-edit-web.md`
- Phase 3: `plans/phase-3-backend.md`, `plans/phase-3-dashboard.md`, `plans/phase-3-l2-model.md`
- Phase 4: `plans/phase-4-adapters.md`, `plans/phase-4-scanner-questions.md`, `plans/phase-4-tui-github-windows.md`
- Cross-cutting: `plans/90-crosscutting-gates-ux-decisions.md` (§6 gates, §7 UX, §9 decisions, §10 agreement)
- Design tokens: `plans/design-tokens.md` (Variant 1 DECIDED) + [`design-tokens.css`](design-tokens.css)

Gates: [`docs/exit-gate-P0.md`](docs/exit-gate-P0.md) · Org plan: [`docs/github-org-plan.md`](docs/github-org-plan.md)

## Non-negotiables

1. Fail-safe → `ask`, never `allow` on error/timeout/crash/parse-fail.
2. Deterministic rules outrank models.
3. Latency budgets in CI — over-budget = no merge (or ADR).
4. Local-first, privacy-by-default (redact before egress, `local-only/redacted/full`), explainable (`algo why`), reversible install, provider abstraction, no invented APIs (`[VERIFY]`).

## Status (Phase 0)

- [ ] proto v0 tagged
- [ ] dataset v0.1 tagged
- [ ] harness + baselines green
- [ ] Jev multi-region report published
- [ ] gate signed, Phase 1 unblocked

See [`docs/exit-gate-P0.md`](docs/exit-gate-P0.md) for the full Phase-1 readiness checklist.
