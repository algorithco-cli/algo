## Summary

Bind Guard organization authorization to the verified Algorithco account
`sub`, document the missing account membership/role contract, and prove every
org/policy/audit/stats surface rejects cross-owner access in account mode.

Task ID: Step 3.5 / PR3

This PR needs human CODEOWNERS review because it changes and verifies
authorization boundaries. Agent review is not sufficient.

## Assumptions

- D1. The central account service is the only identity provider. Guard's own signup/login/provider flows are replaced, not extended. The Zitadel plan is dropped.
- D2. The Guard backend only VERIFIES account-service tokens (JWKS). It never issues its own session tokens in `account` mode.
- D3. Plans and limits come from the account service entitlements. Guard stores no prices, no subscriptions, no payment data.
- D4. Rollout is dual-mode behind `GUARD_AUTH_MODE=legacy|account` (default `legacy` until the final PR of this task, which flips the default to `account`). Legacy code is REMOVED later (Step 5), not in this task.
- D5. Local-first is unchanged: the CLI, agent and core must work fully offline and without any account. An account is required only for cloud/team features (policy sync, audit ingest, dashboard, org features).
- D6. Entitlements gate FEATURES only. An entitlement/auth/network failure must never change an allow/ask/deny decision and never weaken the hard-deny list, thresholds, or redaction.
- D7. DEFER the webhook receiver. No SQLite, no new datastore, no change to the account service contract. The receiver will be implemented per the durable-acceptance contract when Guard adopts a real production datastore.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Decision

- The account service does not expose product-facing organization membership
  or role claims/API. Guard keeps the existing local single-owner mapping,
  keyed only by the verified account `sub`. It does not infer membership or
  roles from an entitlement owner.
- Other-owner and unknown organization IDs remain indistinguishable 404s.
- D7 remains in force: no account webhook receiver is registered.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Contract verification

Verified 2026-10-02 against the account repository:

- `apps/api/src/billing/entitlement-service.ts` uses memberships and billing
  seats internally while computing effective entitlements.
- `apps/api/src/billing/entitlements.ts` can return an entitlement owner of
  type `user` or `org`; it does not define membership or role authorization.
- `apps/api/src/oidc/provider.ts` emits identity and optional compact
  entitlement claims, but no organization membership or role claim.
- The documented public/product routes expose entitlement and billing
  operations, but no product-facing organization membership/role lookup.

## Verification

- `cargo fmt --check`: passed.
- `cargo clippy -- -D warnings`: passed.
- `cargo test`: passed, 134 tests, including account-mode IDOR coverage for
  org create/read/list, policy read/write/dry-run, audit read/write/stream/
  export, and stats.
- `cargo deny check`: passed with existing warnings.
- `cargo audit`: passed with the existing allowed yanked `yoke-derive`
  warning.
- Web: lint passed, 82 tests passed, production build passed.
- Dashboard: lint passed, 11 tests passed, production build passed.
- `buf lint proto` and the main-branch `buf breaking` comparison: passed.
- Gitleaks and the changed-document link check: passed.
