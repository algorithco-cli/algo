# algocli-tui

TUI for algorithco guard — live decision feed, stats, and policy snapshot
(offline-capable, SQLite read-only `~/.algo/audit.db`). The login gate uses the
central account service's device flow on a background thread and stores tokens
only through the native OS credential vault. Continue offline never starts a
network request, and authentication cannot alter policy decisions.

Run: `cargo run -p algocli-tui`. Reads audit DB read-only; never writes decisions.
