# proto — `algorithco_guard.v0` contracts (Phase 0, contracts only)

Product: **algorithco guard** · CLI: **`algo`** · Org: `algorithcoguard`.
Proto package: `algorithco_guard.v0`. No product code in this tree.

## Layout

- `buf.yaml` / `buf.lock` / `buf.gen.yaml` — buf workspace (lint, breaking, codegen).
- `algorithco_guard/v0/events.proto` — `AgentIdentity`, `ToolKind`, `ToolBefore`,
  `ToolAfter`, `AgentStop`, `AgentQuestion` (FREEFORM never auto-answered).
- `algorithco_guard/v0/decision.proto` — `Decision` (error/unknown → ASK, never ALLOW).
- `algorithco_guard/v0/dataset.proto` — `DatasetRecord` (eval imports this; no duplicates).
- `algorithco_guard/v0/hook_daemon.proto` + `daemon_backend.proto` —
  `EXPERIMENTAL — do not implement in Phase 0`.
- `VERSIONING.md` — semver, generate-at-build, exact pins.
- `buf-ci.md` — CI intent; real workflow at `.github/workflows/buf.yml`.
- `CHANGELOG.md` — v0 alpha history.

## Quickstart

```sh
buf lint proto
buf breaking proto --against '.git#tag=v0.0.1-alpha'
buf generate proto --template proto/buf.gen.yaml
```

## Rules

- Contracts-first: `buf breaking` must pass; never duplicate proto-owned types.
- Generate at build from the pinned tag; never hand-edit `gen/` output.
- `[VERIFY]`: unverified hook/Jev fields stay `optional` + commented; never invent
  Jev APIs.
- Fail-safe: error/unknown → ASK, never ALLOW.

## Codegen smoke notes (P0-PROTO-6)

- Template: `buf.gen.yaml` — local plugins `protoc-gen-prost` (Rust) +
  `protoc-gen-es` (TS). Versions pinned in `.github/workflows/buf.yml` env
  (`PROTOC_GEN_PROST_VERSION`, `PROTOC_GEN_ES_VERSION`).
- Local status (2026-09-20, Windows, buf 1.71.0): `buf lint proto` **PASS**;
  `buf generate` blocked — plugins not on `%PATH%`
  (`protoc-gen-prost` / `protoc-gen-es` not installed). Expected green in CI
  after the plugin-install step; re-run locally after
  `cargo install protoc-gen-prost` + `npm i -g @bufbuild/protoc-gen-es`.
- Breaking baseline: no `v*` tag exists yet, so
  `buf breaking --against '.git#tag=<latest>'` has nothing to compare against.
  Cutting `v0.0.1-alpha` establishes the baseline; the first intentional
  breaking change after that must fail CI (acceptance P0-PROTO-1).
- Enum naming: plan shorthand (`SHELL`, `ALLOW`, `SAFE`, …) is rendered with
  protobuf-style prefixes (`TOOL_KIND_SHELL`, `ACTION_ALLOW`, `LABEL_SAFE`, …)
  because enum values share the package scope. Shorthand is kept in comments.
- Package exception: `PACKAGE_VERSION_SUFFIX` lint is exempted in `buf.yaml`
  because `algorithco_guard.v0` is DECIDED (plan §Naming, D-10).
