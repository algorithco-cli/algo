# Phase 4 — Codex + OpenCode Adapters

> Spike rule: each adapter starts with time-boxed [VERIFY] spike vs official docs. If interception cannot block, degrade to observe/advise and cut scope via ADR — never guess-and-allow.

## P4-01 Proto additions (first)

`Finding{rule_id,severity,path,span,redacted_excerpt,reason}`, `agent.question{options[],free_form_bool}`, `QuestionResolution{chosen|deferred}`. Tag, codegen clean.

## P4-02 Codex adapter (`agent/adapters/codex`)

Spike: config approval/sandbox/MCP/notify mapping; which events block vs observe. Then `parse→CanonicalEvent` / `render(Decision)` + degradation matrix + versioning separate from Claude adapter.

AC: headless Codex E2E; unsupported-cap test proves observe-mode, never false-allow.

## P4-03 OpenCode adapter (`agent/adapters/opencode`)

Same pattern vs plugin/config system; confirm blockable events. [VERIFY] plugin events. Separate versioning.

AC: headless OpenCode E2E; graceful degradation tested.

## Shared testing

Scripted scenarios per adapter incl. daemon-down, timeout, corrupt-model. `rmcp` maturity [VERIFY] before MCP server work.
