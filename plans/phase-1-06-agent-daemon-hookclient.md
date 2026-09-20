# P1-06 Agent: daemon + hook-client (`P1-AGENT-1`)

`agent/crates/daemon/src/{main,pipeline,cache,jev_pool,transport}.rs`, `agent/crates/hook-client/src/main.rs`

## Daemon (tokio)

- `UnixListener` `~/.algo/algo.sock` `0600` + `Transport` trait with `NamedPipe` stub for Win (P4).
- Pipeline: L0 policy.evaluate → L1 moka/dashmap cache (blake3 key, TTL 24h, max 10k) → L3 Jev (miss+uncertain) → L4 ask. Attach source, latency_ms.
- Warm Jev h2 pool on start. Single SQLite writer task, WAL, `busy_timeout 5s`; DB locked → ask, never block >1s.

## Hook-client

Tiny binary: `--stdin JSON --socket <uds|pipe> --timeout 1200ms`. `connect_timeout 100ms`, `request_timeout 1000ms`. If daemon down → try `spawn daemon --oneshot` once → else print ask JSON + exit 0 (never ambiguous non-zero — [VERIFY] Claude exit semantics). Startup ~1ms (release, strip).

## Acceptance

- 100 rps loopback L0/L1 p50<3ms.
- Kill-daemon test: hook-client exits 0 with ask/source:fallback.
- Socket perms 0600 asserted.
