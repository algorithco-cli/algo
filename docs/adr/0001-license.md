# ADR-0001: License per repo

## Status

draft (legal sign-off pending)

## Context

Security tool trust depends on license choice. Blocks public release (D-07 / D-B). Need one rule per repo (`proto`, `core`, `agent`, `backend`, `dashboard`, `web`, `eval`).

## Decision

Propose permissive licensing for `core` and `agent` (e.g. Apache-2.0 or MIT — final SPDX after legal review). Other repos default to the same unless backend/dash needs separate terms.

## Alternatives

- **Copyleft (GPL/AGPL)**: strong openness, but adoption friction for a guard injected into commercial workflows — rejected for core/agent.
- **Source-available / proprietary**: keeps control, but harms trust for a security tool — rejected for core/agent.

## Consequences

### Positive

- Easy audit and adoption; matches distributor expectations (Homebrew, package repos).

### Negative

- Forks may ship incompatible policy defaults; mitigate with signed default bundles + trademark on name.

## Verification

- Legal sign-off recorded here before `proto` tag / public release.
- License headers applied per repo after acceptance.
