# ADR-0002: Naming

## Status

accepted

## Context

Repos, crates, proto paths, and home dir depend on stable names (D-10 / D-C, DECIDED in master plan).

## Decision

- Product: `algorithco guard`
- CLI: `algo` (`init|doctor|status|why|log|enforce|pause|login|policy`)
- Proto package: `algorithco_guard.v0`
- Rust crates: `algo-*`
- Home dir: `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db`
- GitHub org: `algorithcoguard`

## Alternatives

- No live alternatives; prior working title `jev-guard` retired to avoid vendor coupling.

## Consequences

### Positive

- Stable import paths and install docs from Phase 0 onward.

### Negative

- `algo` is short; watch for PATH collisions — mitigate with install check in `algo doctor`.

## Verification

- This ADR merged 2026-09-20.
