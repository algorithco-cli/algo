# Contributing — algorithco guard

## Commits

Conventional commits. Examples:

- `docs: add threat-model v0 assets`
- `docs(adr): draft D-01 BYOK vs proxy`
- `chore: fix markdown links`

Types: `docs`, `feat`, `fix`, `chore`, `eval`, `proto` (cross-repo link required).

## PRs

- Small PRs. One repo per PR.
- Link a tracking issue from [tracking-issues.md](./tracking-issues.md).
- Include an `Assumptions:` section if anything was ambiguous.
- Update docs in the same PR as the behavior change.
- No secrets in code, logs, fixtures, or datasets. Pre-commit + CI scan.

## Decisions

- `[DECISION]` items need a merged ADR before implementation code. See [adr/](./adr/README.md).
- Branch name: `adr/D0X-short-title` for decision work.

## Reviews

- Paths in [CODEOWNERS](./CODEOWNERS) need human sign-off.
- Budgets, thresholds, or fail-safe changes need an ADR + human sign-off.
