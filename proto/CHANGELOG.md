# Changelog — proto contracts

## [Unreleased]

- v0 alpha scaffold: `events`, `decision`, `dataset`, experimental wire stubs.

## v0.0.1-alpha (recommended first tag — not yet cut)

- Initial `algorithco_guard.v0` alpha contracts:
  - `events.proto`: `AgentIdentity`, `ToolKind`, `ToolBefore`, `ToolAfter`,
    `AgentStop`, `AgentQuestion` (FREEFORM never auto-answered).
  - `decision.proto`: `Decision` with fail-safe invariant (error/unknown → ASK).
  - `dataset.proto`: `DatasetRecord` with required `redaction_cert`.
  - `hook_daemon.proto` + `daemon_backend.proto`: EXPERIMENTAL placeholders,
    not for implementation in Phase 0.
- buf workspace: `buf.yaml` (v2), `buf.gen.yaml` (Rust prost + TS es),
  CI `buf lint` + `buf breaking --against '.git#tag=<latest>'`.
- Tag procedure: see `VERSIONING.md`. Consumers pin exact tag.
