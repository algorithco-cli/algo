# P1-07 Agent: Claude adapter shell-only (`P1-AGENT-2`)

`agent/crates/adapter-claude/src/{parse,render}.rs`

## parse

Claude PreToolUse JSON for Bash (`tool_input.command`, `cwd`, `session_id`) → `CanonicalEvent{tool_kind:shell}`. Ignore Edit/Write/Read in P1 (log `skipped:unsupported_tool` + ask passthrough). Pure function, no policy logic. Redact before logging.

## render

- allow → `{"decision":"approve"}`, deny → `{"decision":"block","reason":…}`, ask → `{"decision":"ask",…}` — exact strings behind [VERIFY] vs current Claude docs; version-gate `adapter_version` in output.
- Unknown schema version → ask + `reason:unsupported_schema`.

## Acceptance

Golden tests for 20 real hook payloads (allow/deny/ask each); fuzz `fuzz_adapter_parse` no panic; [VERIFY] refresh each Claude release.
