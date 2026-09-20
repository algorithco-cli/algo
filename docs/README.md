# algorithco guard — Docs (Phase 0)

Product: **algorithco guard**. CLI: **`algo`**. Proto: `algorithco_guard.v0`. Crates: `algo-*`. Home: `~/.algo/`.

Org: `algorithcoguard` on GitHub.

This folder is the org-level source of truth for decisions, security, and process.
Product repos (`proto`, `core`, `agent`, `backend`, `dashboard`, `web`, `eval`) each have their own `AGENTS.md`.

## Map

- [AGENTS.md](./AGENTS.md) — how to work in this repo (commands, gates).
- [CONTRIBUTING.md](./CONTRIBUTING.md) — commits, PRs.
- [CODEOWNERS](./CODEOWNERS) — human review paths.
- [roadmap.md](./roadmap.md) — Phase 0-4 build order.
- [decision-log.md](./decision-log.md) — D-01..D-10 + D-A..D-F.
- [threat-model-v0.md](./threat-model-v0.md) — assets, boundaries, abuse cases.
- [security-model.md](./security-model.md) — fail-safe, rules outrank models.
- [privacy-dataflow.md](./privacy-dataflow.md) — local-only / redacted / full.
- [tracking-issues.md](./tracking-issues.md) — one issue per P0 workstream.
- [adr/](./adr/README.md) — Architecture Decision Records.

## Source plans

Plans are the build source of truth (outside this folder):

- `../plans/00-index-build-order.md`
- `../plans/phase-0-01-docs-adr-threat-model.md`
- `../plans/90-crosscutting-gates-ux-decisions.md`
- `../jev-guard-project-plan.md`

> Fresh-agent path: read this file, then [adr/](./adr/README.md), then `../plans/00-index-build-order.md`. Target < 2 min to find plan + ADR dir.

## Phase 0 scope

Docs only. No product code. No L2 design beyond ToS note. No infra.
