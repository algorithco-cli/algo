# Security model (stub — Phase 0)

## Fail-safe: ask, never allow

Any error, timeout, crash, missing model, parse failure, or unreachable service resolves to `ask`. Never `allow`.

Every new I/O / timeout / parse path needs a `proves_ask_on_*` test. Allow-on-error mutants are killed in review.

## Rules outrank models

Order: L0 hard rules -> L1 cache -> L2 local classifier -> L3 Jev -> L4 user.

- Hard deny runs on the syntax tree, not raw text. It cannot be overridden by Jev, learned preferences, or `always-allow` user rules.
- `always-allow` records a local rule or training signal only; hard-deny override is rejected + tested.
- Threshold changes need an ADR + human sign-off (see [CODEOWNERS](./CODEOWNERS)).

## Install integrity

- `algo init` shows exact file diffs, asks per-agent consent, backs up configs, adds hooks without removing existing ones.
- `algo uninstall` restores from backup. `algo pause` works even daemon-broken.
- Releases are signed with SBOM + provenance; the daemon verifies signatures (policy bundles, L2 artifacts, updates).

## Audit

Each decision records action + reason + confidence + source + latency. Local SQLite `~/.algo/audit.db`. `algo why` prints the latest.
