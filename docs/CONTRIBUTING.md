# Contributing — algorithco guard

## Commits

Conventional commits. Examples:

- `docs: add threat-model v0 assets`
- `chore: fix markdown links`

Types: `docs`, `feat`, `fix`, `chore`, `eval`, `proto` (cross-repo link required).

## PRs

- Small PRs. One repo per PR.
- Include an `Assumptions:` section if anything was ambiguous.
- Update docs in the same PR as the behavior change.
- No secrets in code, logs, fixtures, or datasets. Pre-commit + CI scan.

## Decisions

- `[DECISION]` items need an explicit owner decision recorded in the PR description before implementation code.

## Reviews

- Paths in [CODEOWNERS](./CODEOWNERS) need human sign-off.
- Budgets, thresholds, or fail-safe changes need owner decision + human sign-off.
