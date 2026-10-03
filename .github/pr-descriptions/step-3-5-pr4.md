## Summary

Replace Guard's web login and billing surfaces in account-mode builds with the
central Algorithco OIDC and entitlement flows while preserving the legacy UI
behind the rollout flag.

Task ID: Step 3.5 / PR4

This PR needs human CODEOWNERS review because it changes browser authentication
and token handling. Agent review is not sufficient.

## Assumptions

- D1. The central account service is the only identity provider. Guard's own signup/login/provider flows are replaced, not extended. The Zitadel plan is dropped.
- D2. The Guard backend only VERIFIES account-service tokens (JWKS). It never issues its own session tokens in `account` mode.
- D3. Plans and limits come from the account service entitlements. Guard stores no prices, no subscriptions, no payment data.
- D4. Rollout is dual-mode behind `GUARD_AUTH_MODE=legacy|account` (default `legacy` until the final PR of this task, which flips the default to `account`). Legacy code is REMOVED later (Step 5), not in this task.
- D5. Local-first is unchanged: the CLI, agent and core must work fully offline and without any account. An account is required only for cloud/team features (policy sync, audit ingest, dashboard, org features).
- D6. Entitlements gate FEATURES only. An entitlement/auth/network failure must never change an allow/ask/deny decision and never weaken the hard-deny list, thresholds, or redaction.
- D7. DEFER the webhook receiver. No SQLite, no new datastore, no change to the account service contract. The receiver will be implemented per the durable-acceptance contract when Guard adopts a real production datastore.
- The account integration docs do not publish a billing-page or checkout URL. `VITE_ACCOUNT_BILLING_URL` is therefore required configuration and `.env.example` leaves it as an explicit owner TODO rather than inventing a path.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Decision

- `guard-web` uses authorization code + PKCE S256 as a public client, with
  random one-shot state/nonce/verifier values and cryptographic ES256 ID-token
  validation against discovered JWKS.
- Account tokens remain in JavaScript process memory only. Login transaction
  values in session storage contain no token. Reload uses a top-level OIDC
  redirect; logout clears local credentials before discovered end-session
  navigation.
- Account-mode pricing and billing render the backend's effective entitlement,
  contain no local prices/card controls, and link only to the configured account
  billing URL. A configured `account_return=1` redirect triggers caller-only
  entitlement refresh.
- D7 remains in force: no account webhook receiver is registered.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Contract verification

Verified 2026-10-02 against the account repository:

- `docs/INTEGRATION.md` requires discovery, exact redirect URI, authorization
  code + PKCE S256, random state and nonce, and scopes
  `openid profile email offline_access`.
- `apps/api/src/oidc/clients.ts` registers `guard-web` as a public web client,
  token endpoint authentication `none`, ES256 ID tokens, authorization-code and
  rotated-refresh grants.
- The account `.env.example` uses
  `https://guard.algorithco.com/auth/callback` as the production redirect and
  `https://guard.algorithco.com` as the allowed browser origin.
- Web Crypto ECDSA signing/verification and key import were checked against the
  [W3C Web Cryptography API Recommendation](https://www.w3.org/TR/WebCryptoAPI/)
  on 2026-10-02. PKCE behavior was checked against
  [RFC 7636](https://www.rfc-editor.org/rfc/rfc7636) on 2026-10-02.

## Verification

- `pnpm lint`: passed.
- `pnpm test`: passed, 95 tests, including PKCE, state/nonce, callback errors,
  expired ID tokens, memory-only token handling, entitlement refresh, and
  logout ordering.
- Default-legacy and explicit account-mode `pnpm build`: passed; the static
  callback route is prerendered and no new third-party scripts are present.
- Backend `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`:
  passed (134 tests).
- `cargo deny check`: passed with existing warnings; `cargo audit`: passed
  with the existing allowed yanked `yoke-derive` warning.
- Dashboard lint, 11 tests, and production build: passed.
- `buf lint` and `buf breaking` against main: passed; proto is unchanged.
- Gitleaks and changed-document link checks: passed.
