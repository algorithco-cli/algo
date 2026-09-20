# ADR-0005: Core distribution (git tags default)

## Status

draft — 2026-09-20 (D-D; owner @algorithcoguard/core)

## Context

`core/` shared libs (`algo-*`: types, shell-analysis, policy, redact, provider, fingerprint)
are consumed by `agent/`, `backend/`, and `eval/` schemas in the `algorithcoguard/algorithco-guard`
monorepo (see `docs/github-org-plan.md` §4). Phase 1 needs a distribution rule before consumer
wiring. No private registry exists in Phase 0.

## Decision

Distribute `core` via monorepo git tags by default. Consumers reference the exact tag/SHA
and generate proto-owned types at build; never commit hand-edited generated code and never
duplicate proto-owned types by hand. Revisit only via a superseding registry ADR.

## Alternatives

- **Publish to crates.io immediately**: rejected for Phase 0/1. Public registry release
  implies license + API-stability commitments the license ADR (D-B/D-07) has not cleared.
- **Stand up a private registry now**: rejected as premature. Operational cost with no
  consumer outside the monorepo yet; revisit when `backend/`/`dashboard/` need it.
- **Git submodules / vendored copies per consumer**: rejected. Duplicates sources of truth
  and breaks contracts-first (proto tag → generate at build).

## Consequences

### Positive

- Zero infra: tags already exist for proto releases; consumers pin exact versions.
- Single source of truth preserved; reproducible builds from a known SHA.
- No license-registry coupling before legal sign-off.

### Negative

- No semver enforcement beyond tag discipline; breaking changes rely on review + `buf breaking`.
- External consumers (future) get no registry ergonomics until the follow-up ADR.
- Tag-namespace hygiene required (monorepo tags version the whole tree).

## Open questions (honest, unresolved)

- Tag scheme: plain `v0.x.y` on the monorepo vs prefixed `core-v*` / `proto-v*` — undecided;
  this draft assumes plain monorepo semver with the generating SHA recorded by consumers.
- SBOM/provenance expectations for git-tagged deps (SLSA scope for giltags vs registry).
- Trigger for revisiting: what consumer pain or release need justifies a registry ADR.

## Verification

- A consumer PR records the exact monorepo tag/SHA it builds against; clean-checkout build
  reproduces without network beyond the pinned tag.
- `buf lint` + `buf breaking` green on any `proto/` change in the same tag line.
- No LICENSE published and no registry publish step in CI until D-B/D-07 clears.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/core` + `@algorithcoguard/security` — required before
  Phase 1 consumer wiring pins a distribution mechanism. Without it, no `core` consumer
  integration and no registry publish setup.