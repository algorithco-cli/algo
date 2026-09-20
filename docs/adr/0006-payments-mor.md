# ADR-0006: Payments MoR (external, deferred; entitlement field reserved)

## Status

draft — 2026-09-20 (D-E / D-08; owner @algorithcoguard/backend)

## Context

Monetization needs a Merchant of Record (MoR) decision, but Phase 0 builds no payment paths
(source: `plans/90-crosscutting-gates-ux-decisions.md` §3 D-08; Phase 0 draft D-E). Deciding
early avoids baking billing assumptions into P3 auth/policy schemas while keeping P0 free
of payment code and secrets.

## Decision

Defer to an external MoR; decide the vendor early (before P3 backend scope), build nothing
in P0–P2, and reserve an `entitlement` field in P3 protocol/policy schemas so enforcement
has a place to land without a breaking change.

## Alternatives

- **Build in-house billing now**: rejected. PCI, tax, and fraud scope dwarfs a guard team;
  no payment code or secrets in Phase 0.
- **Stripe-direct (or equivalent) inside `backend/` now**: rejected as premature. It skips
  the MoR question (tax liability, invoicing, regional compliance) this ADR defers.
- **No entitlement field (add it when billing lands)**: rejected. Adding enforcement
  later would force a breaking proto change; reserving an optional field now is cheap.

## Consequences

### Positive

- P0–P2 stay payment-free: no secrets, no PCI surface, no billing branches to fail open.
- P3 can enforce entitlements without breaking contracts (optional field, fail-safe default).
- Vendor choice stays reversible until P3 backend scope starts.

### Negative

- Entitlement semantics (tiers, offline grace, tamper response) stay undefined until P3.
- Reserved-but-unenforced field risks confusion; docs must mark it inert until P3.
- External-vendor dependence (fees, policy, availability) accepted without analysis here.

## Open questions (honest, unresolved)

- Which external MoR vendor, and what criteria (fees, tax coverage, API, privacy) decide.
- Entitlement semantics: field shape, offline behavior, downgrade/expiry mapping to
  allow/ask/deny — must fail safe (unknown/expired → `ask`, never silent `allow`).
- Interaction with BYOK (ADR-0004) quotas and team/proxy billing, if a proxy ever lands.
- No LICENSE or pricing terms decided here (license ADR D-B/D-07 is separate, legal-gated).

## Verification

- Grep of P0–P2 trees shows no payment/MoR code, keys, or fixtures; gitleaks green.
- P3 schema review confirms a reserved optional `entitlement` field documented as inert
  until the follow-up billing ADR lands; unknown values prove `ask`.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/backend` + `@algorithcoguard/legal` (+ `@algorithcoguard/security`
  for enforcement posture) — required before any MoR vendor commitment or any payment/
  entitlement enforcement code. Without it, no billing build and no entitlement semantics change.