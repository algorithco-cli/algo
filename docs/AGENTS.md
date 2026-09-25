# AGENTS.md — docs repo

## Scope

Markdown only. No product code. Small files. No secrets.

## Runnable commands

These are the real gates used across repos. Run the subset that applies to your change:

```powershell
# proto contracts
buf lint
buf breaking --against .git#branch=main

# rust (core, agent, backend)
cargo test
cargo clippy -- -D warnings
cargo deny check
cargo audit

# python (eval)
pytest
ruff check .
mypy .

# markdown links (docs, web)
# markdown-link-check version/pin — 2026-09-20 — https://github.com/tcort/markdown-link-check
npx --yes markdown-link-check docs/**/*.md
```

## Conventions

- One repo per PR. Link a tracking issue from [tracking-issues.md](./tracking-issues.md).
- ADR before code for any `[DECISION]`. See [adr/](./adr/README.md).
- Fail-safe: any doubt resolves to `ask`, never `allow`.
- Redact before network. Telemetry opt-in, never code.

## Before you claim done

1. `buf lint` green if `proto` touched (not in this repo — cross-link the proto PR).
2. Markdown links resolve (relative links only, no invented paths).
