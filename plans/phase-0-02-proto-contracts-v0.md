# P0-02 Proto v0 Contracts

Tooling: buf (lint, breaking, codegen), Protobuf, ConnectRPC/gRPC.

## 1. buf workspace + CI (`P0-PROTO-1`)

- `buf.yaml`, `buf.lock`, `buf.gen.yaml`, CI `buf lint` + `buf breaking --against '.git#tag=<latest>'`.
- `VERSIONING.md`: semver, generate-at-build from tag, exact pins, never hand-edit generated code.
- Acceptance: CI green; demo breaking PR fails `breaking`.

## 2. Canonical events v0 (`P0-PROTO-2`)

`proto/algorithco_guard/v0/events.proto`:

- `AgentIdentity{agent_type, agent_version, session_id, working_dir}`
- `ToolKind{SHELL,EDIT,WRITE,READ,NET,OTHER}`
- `ToolBefore{event_id, timestamp, agent, tool_kind, redacted_payload, privacy_mode, optional shell_argv/file_path}`
- `ToolAfter`, `AgentStop`, `AgentQuestion{CHOICE/FREEFORM}` — comment: FREEFORM never auto-answered.
- Unverified agent fields = `optional` with `[VERIFY]` comment, never guessed-required.
- Acceptance: `buf lint` passes, every message has doc comments.

## 3. Decision v0 (`P0-PROTO-3`)

`decision.proto`: `Decision{action ALLOW/DENY/ASK, reason (one line for why), confidence_0_1, source_level RULE/CACHE/LOCAL_MODEL/JEV/FALLBACK, latency_ms, policy_version, trace_id}` + invariant: error/unknown → `ASK`, never `ALLOW`.

## 4. Dataset record v0 (`P0-PROTO-4`)

`dataset.proto`: `DatasetRecord{record_id, canonical ToolBefore, label SAFE/DANGEROUS/AMBIGUOUS, rationale, obfuscation NONE/VAR_EXPANSION/ENCODING/SUBSHELL/PIPE_CHAIN/OTHER, annotator, dataset_version, redaction_cert}` + version manifest. `redaction_cert` required. `eval` imports this, no duplicated struct.

## 5. Wire stubs v0alpha (`P0-PROTO-5`)

`hook_daemon.proto` + `daemon_backend.proto` marked `// EXPERIMENTAL — do not implement in Phase 0` (auth/policy-sig/audit placeholders, redacted+opt-in notes). Nothing consumes them.

## 6. Codegen smoke + tag (`P0-PROTO-6`)

CI generates Rust + TS from tag; scratch-consumer `cargo check` + `npm run build`; tag + changelog. Consumers pin exact tag.

## Naming — DECIDED

Product `algorithco guard`, CLI `algo`, proto package `algorithco_guard.v0`. No interim reservation needed.
