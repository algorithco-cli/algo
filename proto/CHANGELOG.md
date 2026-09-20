# Changelog — proto contracts

## [Unreleased]

- v0 alpha scaffold: `events`, `decision`, `dataset`, experimental wire stubs.
- 2026-09-20: `dataset.proto`: `redaction_cert` (field 8) becomes
  `RedactionCert { bool redacted = 1; string scanner = 2; string notes = 3; }`,
  documented REQUIRED (eval enforces absence → reject). Aligns proto with the
  eval object shape per ADR-0003; eval JSON adopts `canonical` naming
  (eval side, no proto field rename).

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
