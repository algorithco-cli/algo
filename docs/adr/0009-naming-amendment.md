# ADR-0009: Naming Amendment — `algo guard` dispatcher (Draft, do not apply yet)

## Status

draft — 2026-09-20 (owner: @algorithcoguard/maintainers + legal)
**Do not apply until owner approves.** This is a Draft amendment to ADR-0002 (accepted).

## Context

ADR-0002 (accepted 2026-09-20) decided:

- Product `algorithco guard`, CLI `algo` (`init|doctor|status|why|log|enforce|pause|login|policy`),
  proto `algorithco_guard.v0`, crates `algo-*`, home `~/.algo/`, socket `~/.algo/algo.sock`,
  DB `~/.algo/audit.db`, org `algorithcoguard`.

The master plan (`jev-guard-project-plan.md:4` and owner feedback 2026-09-20) now
requires a **shared `algo` dispatcher** for AlgorithCo Voice/Tunnel, with this product
as a subcommand:

- CLI: **`algo guard <command>`** (i.e., `algo guard init`, `algo guard doctor`, etc.; the top-level `algo` is the AlgorithCo dispatcher, not this product alone).
- Binaries: **`algo-guard`** (daemon + CLI entry, replaces `algo`) and **`algo-guard-hook`** (hook client, replaces `algo-hook`/`algo_*`).
- GitHub org: **`algorithco`** (not `algorithcoguard`).
- Repo names: **`guard-*`** (e.g., `guard-proto`, `guard-core`, `guard-agent`, `guard-backend`, `guard-dashboard`, `guard-web`, `guard-eval`, `guard-docs`) — or the existing monorepo `guard` with directory prefixes.
- Rust crates: **`algorithco-guard-*`** (e.g., `algorithco-guard-types`, `algorithco-guard-policy`) — replaces `algo-*`.
- Home dir / socket / DB: **still `~/.algo/`** **unless** the dispatcher owns `~/.algo/` and this product moves to `~/.algo/guard/` or `~/.guard/` — **decision needed** (see Alternatives).

This ADR is the **conflict flag per `AGENTS.md:5`** (plan vs AGENTS.md — stop + ask). No code or doc is changed by this file alone.

## Decision (Proposed — Draft, not applied)

If approved, adopt the plan's naming:

- CLI entry: `algo guard` (subcommand), help text `algo guard --help`, shell completion `algo guard`.
- Binaries: `algo-guard` (installed as `algo-guard`, also available via `algo guard` dispatcher), `algo-guard-hook`.
- Org: `algorithco`; repos: `guard-*` (or monorepo `algorithco/guard` with `guard-*` subdirs as in the split plan).
- Crates: `algorithco-guard-*`.
- Keep proto package `algorithco_guard.v0` **unless** the org rename drives a proto rename to `guard.v0` — **deferred** (buf breaking concern; see Alternatives).
- Keep `~/.algo/` **unless** dispatcher owns it — then move to `~/.algo/guard/` (or `~/.guard/`) with a migration shim (`AGENTS.md:97`).

## Alternatives

- **Keep ADR-0002 as-is (`algo` top-level, `algo-*` crates, `~/.algo/`, org `algorithcoguard`)**: rejected if the `algo` dispatcher is real — it would collide with Voice/Tunnel and waste the org `algorithco`. Keep only if dispatcher is speculative — then this amendment stays Draft and `algo` remains.
- **Rename CLI but keep crates `algo-*`**: rejected — crate and CLI divergence confuses `cargo install algo-guard` vs `cargo install algo`.
- **Rename home dir to `~/.guard/` unconditionally**: rejected without dispatcher ownership proof — it breaks existing `algo init` expectations and the `design-tokens.css` / `~/.algo/` references in `AGENTS.md:97` and `plans/phase-1-08-agent-cli-audit-shadow.md:5`.

## Consequences (if approved)

- Every reference listed below must change (one PR per repo, conventional commits, `agent-changelog/` entry).
- Install docs (`web` + `README.md`), `algo doctor` PATH checks, and `algo init`/`algo uninstall` backup paths change.
- `cargo install` name changes (`algo-guard` vs `algo`); Homebrew tap / winget / apt package names change.
- Proto package rename (if any) is a **breaking buf change** (needs `buf breaking` major bump + consumer migration).
- Org `algorithcoguard` → `algorithco` requires org transfer or new org + repo moves + remote URL updates.

### Positive

- No collision with `algorithco` Voice/Tunnel dispatcher; single `algo` entry point for the AlgorithCo suite.
- Binary names `algo-guard` / `algo-guard-hook` are unambiguous on PATH and in logs.

### Negative

- Renames touch **every** doc, script, and crate manifest; `git log` will show a coherent rename commit if done with `git mv`.
- Users who ran `algo init` (Phase 1 stub) would need a migration (`~/.algo/` → `~/.algo/guard/` or `~/.guard/` if changed).

## Files to change (if approved) — exhaustive list as of 2026-09-20

No file is changed by this Draft file itself. This is the inventory for the future rename PR(s):

- **ADRs / decision log:** `docs/adr/0002-naming.md` (superseded by this ADR if accepted), `docs/adr/README.md` index, `docs/decision-log.md` D-10/D-C rows.
- **Working agreements:** `AGENTS.md:4,93-97,62` (Naming, UX checklist `algo init`, Runnable commands), `docs/AGENTS.md`, `docs/CONTRIBUTING.md`, `docs/roadmap.md`, `docs/README.md`, `docs/github-org-plan.md` (org + repo names + `gh repo create` commands), `docs/exit-gate-P0.md` (UX checklist + CLI examples), `docs/threat-model-v0.md` (home dir / socket / DB paths), `docs/privacy-dataflow.md` (CLI examples), `docs/redact-consent-readiness.md` (`algo log --show-egress`, `algo init` examples), `docs/STATUS-2026-09-20.md` (CLI/album references).
- **Proto:** `proto/buf.yaml` (package `algorithco_guard.v0` if renamed), `proto/README.md`, `proto/AGENTS.md`, `proto/VERSIONING.md`, `proto/CHANGELOG.md`, `proto/algorithco_guard/v0/*.proto` (package line), `proto/buf.gen.yaml`, `.github/workflows/buf.yml` (paths).
- **Eval:** `eval/pyproject.toml` (name `algorithco-guard-eval` if crates renamed), `eval/AGENTS.md`, `eval/README.md`, `eval/harness/*`, `eval/jev_client/README.md`, `eval/ci-eval.md`.
- **Agent / core / backend scaffolds:** `core/README.md`, `agent/README.md`, `backend/README.md`, `dashboard/README.md`, `web/README.md`, `core/crates/*` / `agent/crates/*` `Cargo.toml` crate names (`algo-*` → `algorithco-guard-*`), `agent/crates/daemon` socket constant (`~/.algo/algo.sock` → `~/.algo/guard.sock` or `~/.guard/...` if home moves), `agent/crates/cli-audit` clap `name = "algo"` → `name = "algo-guard"` + subcommand `guard` handling if dispatcher delegates.
- **CI / scripts:** `.github/PULL_REQUEST_TEMPLATE.md`, `.github/workflows/*.yml` (job names, `algo` binary names), `design-tokens.css` comments (if they mention `algo`), `agent-changelog/*`.
- **Master plan / plans:** `jev-guard-project-plan.md:4` + `plans/00-index-build-order.md:4` (CLI line), `plans/phase-1-*` (all `algo ...` examples: `phase-1-06-agent-daemon-hookclient.md:11`, `phase-1-08-agent-cli-audit-shadow.md:7-14`, `phase-1-10-exit-gate-runbook.md:9`), `plans/90-crosscutting-gates-ux-decisions.md` §10 (CLI `algo`).

## Verification (what would prove the amendment landed correctly)

- `grep -R "algo init|algo doctor|algo-\*|~/.algo" --include="*.md" --include="*.toml" --include="*.rs"` shows only the new names (`algo guard init`, `algo-guard`, `algorithco-guard-*`, `~/.algo/guard/` or `~/.guard/` if moved) except for historical changelog entries.
- `cargo test` (once crates exist) still passes — crate rename is mechanical via `cargo rename` / `git mv`.
- `buf lint` + `buf breaking` still pass; if proto package renamed, a **major** `buf breaking` bump + migration note is published.
- `algo guard --help` and `algo-guard --help` both work (dispatcher + direct binary).
- Install script (`web` landing) installs `algo-guard` and registers the `algo guard` subcommand without colliding with Voice/Tunnel.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/maintainers` + `@algorithcoguard/legal` (naming/trademark) — required before any rename PR(s). Without it, ADR-0002 stays controlling and `algo` / `algo-*` / `~/.algo/` remain.
- [ ] If approved: owner confirms home dir stays `~/.algo/` vs moves to `~/.algo/guard/` or `~/.guard/` (one line).
