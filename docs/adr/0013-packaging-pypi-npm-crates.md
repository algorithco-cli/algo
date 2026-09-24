# ADR-0013: Packaging for crates.io / PyPI / npm (private MVP prep)

## Status

draft (blocks public release; does not authorize `publish` while repo private per ADR-0009)

## Context

Monorepo ships Rust (`algo-*`), Python eval harness, TS SPAs + future contracts SDK.
All versions `0.1.0`. License `LicenseRef-TBD-legal-signoff` (ADR-0001 draft).
Repo private (`algorithcoguard/algorithco-guard`). Need crates.io / PyPI / npm
readiness without publishing. See `plans/00-index-build-order.md`, `proto/VERSIONING.md`.

## Decision

- Rust: publishable in topological order types/redact/shell-analysis/hook-client/backend
  → fingerprint/policy/audit → provider/adapters → daemon/cli/verifier/loop/scanner/tui.
  `algo-policy-spike` + `fuzz` are `publish=false`. All internal path deps carry
  `version="0.1.0"`. Keywords + categories added. License stays TBD until ADR-0001 sign-off.
- Python: only `algorithco-guard-eval` is distributable. `README.md` is landing page
  (not `AGENTS.md`). `authors`, `urls`, `classifiers`, `[project.scripts] eval-harness`,
  `jev` extra single-sources `httpx`. `tests*` + `jev_client/` excluded from wheel.
  Mark `Development Status :: 1 - Planning` + do not upload while private.
- npm: `dashboard`, `web` stay `private:true` + `UNLICENSED` (static SPAs, never publish).
  Only publishable is `@algorithco/guard-contracts` scaffold in `packages/contracts/`
  (protobuf-es output, `files:["dist"]`, `publishConfig provenance`, pinned proto tag
  `v0.0.1-alpha`, `gen/` never committed).

## Alternatives

- **Publish SPAs to npm**: rejected — Vite static sites have no entry points; ship via hosting, not registry.
- **Publish eval + jev_client together**: rejected — `jev_client/` is throwaway probe
  without package structure; keep excluded until it gains `__init__.py` + relative imports.
- **Pick MIT now to unblock publish**: rejected — license needs legal sign-off per ADR-0001; use `UNLICENSED` / TBD placeholders.

## Consequences

### Positive

- `cargo publish --dry-run`, `twine check`, `npm pack --dry-run` pass without network publish.
- Single-source deps; no path-only publish failure; no missing-README failure.

### Negative

- Still blocked on legal (SPDX), public repo, Trusted Publisher + SLSA/SBOM wiring before any real upload.

## Verification

- `cargo package --list --allow-dirty -p algo-types/fingerprint/policy` green in `core/`;
  `-p algo-tui/cli` green in `agent/` (tui README fix confirmed) — 2026-09-23.
- `cargo publish --dry-run -p algo-types` packages 7 files OK, but `verify` fails in
  `build.rs:25` (proto path `../../../proto` outside tarball) — pre-existing
  contracts-first gap, needs follow-up (vendor protos or generated-code fallback).
  License `LicenseRef-TBD` also still blocks real publish (ADR-0001).
- `python -m build && twine check` PASSED in `eval/` — wheel excludes `tests*` +
  `jev_client`, includes `entry_points.txt` (`eval-harness`) — 2026-09-23.
  `ruff check` + `mypy` clean, `pytest tests/` 13 passed
  (`PYTEST_DISABLE_PLUGIN_AUTOLOAD=1`; env `web3/eth_typing` plugin broken pre-existing).
- `npm pack --dry-run` green in `packages/contracts/` (2 files scaffold) — 2026-09-23.
- CI must add: `cargo publish --dry-run` topological, PyPI Trusted Publisher, npm provenance.
- Assumptions: first public versions stay `0.1.0`; scope `@algorithco` proposed, not reserved.
