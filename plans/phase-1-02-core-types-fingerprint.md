# P1-02 Core: types + fingerprint (`P1-CORE-1`)

## types (`core/crates/types/`)

- Re-export proto-generated `CanonicalEvent{agent_id,session_id,cwd,tool_kind:shell,command,timeout?}`, `Decision{action,reason,confidence,source:rule|cache|jev|fallback,latency_ms}` + helpers (`is_deny()`, `to_ask_on_error()`).
- Pin proto tag in Cargo.toml, codegen at build via prost-build, never hand-edit.
- AC: `cargo build -p algo-types` passes; tag in Cargo.lock; round-trip Event→JSON→Event test.

## fingerprint (`core/crates/fingerprint/`)

`normalize(cmd: &ParsedCmd) -> String`: lowercase argv[0] basename, sort order-invariant long flags, replace `/(tmp|var|home)/[^ ]+`, `[0-9a-f]{7,}`, timestamps, uuid → `<PATH>/<HASH>/<NUM>`, preserve redirections/pipes/sudo. `cache_key = blake3(normalize + policy_version + profile)`.

Edges: `VAR=x cmd`, `sudo -u u cmd`, `cmd 2>&1 | tee`, quoted vs unquoted, `sh -c '...'` one level only — deeper → ask + `unparseable:nested`.

AC: proptest `normalize(normalize(x))==normalize(x)`; never emits secret-looking substrings (checked vs redact); 200-case corpus (`curl …/a1` vs `…/d4` same key; `rm -rf /` vs `rm -rf /tmp/x` different).
