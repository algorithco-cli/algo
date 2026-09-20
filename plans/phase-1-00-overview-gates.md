# Phase 1 Overview — Local MVP (No Cloud)

> Observes Claude Code Bash calls, decides allow/deny/ask via L0→L1→L3→L4. No backend/dashboard, no L2, no verifier/loop (P2), no Codex/OpenCode (P4). Ends with shadow soak.

## Requirements (from §2,§4,§5.2,§5.3,§7,§8)

- R1 Core: types, shell-analysis (tree-sitter-bash), policy, redact, provider (Jev+mock), fingerprint.
- R2 Agent: hook-client (~1ms), daemon (tokio, UDS + pipe abstraction), Claude shell-only, SQLite WAL audit, CLI `algo` (`init|uninstall|doctor|pause|resume|status|why|log|policy`), shadow default.
- R3 Fail-safe: error/timeout/crash/parse-fail → ask. Hard-deny outranks all.
- R4 Quality: fuzz + proptest + mutants (≥90% deny) + regression incl. obfuscated + clippy/deny/audit clean + eval gate.
- R5 Perf: L0/L1 <3ms p50/<10ms p99 CI-enforced; L3 <250/<800ms.
- R6 UX: `algo init` <30s, consent-per-agent, backup + additive merge, `algo uninstall` byte-identical, `algo pause` daemon-dead, `algo why` one-line, quiet.
- R7 Exit: benchmarks green, eval pass, macOS+Linux matrix, ≥3 users × ≥3 days shadow.

## Build order

```
P1-00 pin contracts → P1-01 ADR CEL/DSL (blocks policy)
 → fingerprint → shell-analysis → policy+deny → redact → provider
 → daemon+hook → claude-shell → audit+CLI+shadow → quality → exit runbook
```

Assumes proto tag + eval harness from P0 pinned. If missing, stub behind cfg + [VERIFY] and block L3.

## Files 01–10

See `phase-1-01-*` through `phase-1-10-*`.
