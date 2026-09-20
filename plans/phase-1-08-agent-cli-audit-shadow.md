# P1-08 Agent: audit + CLI + shadow (`P1-AGENT-3`)

## Audit (`agent/crates/audit/`)

SQLite WAL `~/.algo/audit.db`: `decisions(ts, session_id, tool_kind, redacted_command, fingerprint, action, source, reason, confidence, latency_ms, profile, shadow bool)`. Never raw secrets.

## CLI (`algo init|uninstall|doctor|pause|resume|status|why|log|policy|enforce|login`)

- `algo init`: detect `~/.claude.json`/`settings.json`, show exact diff, per-agent consent, backup `*.algo-backup-<ts>`, additive hook merge, privacy prompt `local-only|redacted|full`, run `algo doctor`.
- `algo uninstall`: restore backup byte-identical, remove socket/db opt-in, remove hooks.
- `algo doctor`: socket, perms, hook present, Jev reachability, latency probe.
- `algo pause`/`algo resume`: touch `~/.algo/paused` — hook-client checks first, instant bypass even daemon-dead.
- `algo why`: last decision pretty (action + reason + confidence + source + latency).
- `algo status`/`algo log`: counts auto-approved/asked/blocked.

## Shadow mode (default P1)

Daemon computes decision but adapter always renders approve + logs `would_have:{allow|deny|ask}`. `algo status` shows `would-have-blocked N`.

## Acceptance

- `algo init→algo doctor→algo uninstall` round-trip on temp HOME restores byte-identical.
- `algo pause` passes daemon-killed.
- Shadow soak ≥500 decisions without ever blocking.
