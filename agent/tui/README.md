# algo-tui

TUI for algorithco guard — live decision feed, stats, and policy snapshot
(offline-capable, SQLite read-only `~/.algo/audit.db`). Authentication and agent
connection changes are intentionally handled by the `algo` CLI, not this viewer.

Run: `cargo run -p algo-tui`. Reads audit DB read-only; never writes decisions.
