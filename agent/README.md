# agent (private MVP — P1 local)

> Part of **algorithco guard** (CLI `algo`). Monorepo: `algorithcoguard/algorithco-guard`.
> Jev stays OFF (MockProvider only).

Scope: on-machine product — hook client, daemon (`~/.algo/algo.sock` 0600), adapters (Claude-first shell-only),
CLI (`algo`), TUI (read-only), audit WAL. Shadow-first; ask on any error.

Workspace members (`agent/Cargo.toml`): `crates/daemon`, `crates/hook-client`, `crates/adapter-claude`,
`crates/audit`, `crates/cli`, `tui`.

## Behavior notes (P1-08)

- `algo pause` touches `~/.algo/paused` (`ALGO_HOME` respected) — `hook-client` checks first,
  instant `allow` bypass even daemon-dead. `algo resume` removes it.
  Any I/O error checking `paused` => NOT paused (proceed to daemon, fail-safe to `ask`).
- Shadow default P1: daemon computes real decision, stores it with `shadow=1`,
  returns `Allow` with `reason: shadow: would_have {deny|ask} → approve (shadow)`.
  JSON adds `shadow` + `would_have`; `algo status` shows `would-have-blocked N`.
  Soak ≥500 without ever blocking. Enforcing still available via unit default.
- `algo enforce off` (shadow, default) / `on` (enforcing) / `status` — writes
  `~/.algo/config.json` (`enforce`/`shadow`); `algo init` preserves prior setting.
  Daemon reads `ALGO_ENFORCE` > `ALGO_SHADOW` > config > default shadow.
  Requires daemon restart (private-MVP limit).
- Fail-safe: enforcing mode daemon unreachable / parse / DB / timeout => `ask`, never `allow`.
  Shadow mode never blocks even on DB error (would_have ask). Hook never exits non-zero.
- TUI is read-only over `audit.db`; crash never blocks hooks.

## Build order

Contracts-first: proto tag, pin exact. `clippy -D warnings`, `cargo test`, latency L0/L1 <3/<10ms.
