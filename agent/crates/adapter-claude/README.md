# algo-adapter-claude

Shell-only Claude adapter (P1-07, `P1-AGENT-2`).

- `parse` (`src/parse.rs`): Claude `PreToolUse` JSON for `Bash` → `CanonicalEvent { tool_kind: shell }`.
  Unsupported tools (Edit/Write/Read) return `Err(SkippedUnsupportedTool)` → caller maps to
  `ask` passthrough with `skipped:unsupported_tool`. Pure, no policy, redacts before logging.
- `render` (`src/render.rs`): `Decision` → Claude hook JSON (`approve`/`block`/`ask`), version-gated
  with `adapter_version`. Unknown `schema_version` → `ask` + `unsupported_schema`.

Payload shapes and decision strings must be re-checked against current Claude docs on each
Claude release.

## Golden tests

`cargo test -p algo-adapter-claude` covers ~20 real hook payloads (safe `ls -la`, dangerous
`rm -rf /`, obfuscated `curl | sh` / `base64 -d`, empty, plus skipped Edit/Write/Read),
render mappings (`approve`/`block`/`ask`), unknown-schema → `unsupported_schema`, and
`proves_ask_on_parse_fail` (never `allow`). `fuzz_adapter_parse` via `proptest` asserts no panic
on arbitrary input.
