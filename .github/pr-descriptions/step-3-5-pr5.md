# Summary

Add native `guard-cli` account login/logout, secure OS-vault credential
storage, refresh-token rotation handling, and the real device-flow TUI while
preserving fully offline local policy behavior.

Task ID: Step 3.5 / PR5

This PR needs human CODEOWNERS review because it changes authentication,
credential storage, install/uninstall behavior, and the Windows install E2E.
Agent review is not sufficient.

## Assumptions

- D1. The central account service is the only identity provider. Guard's own signup/login/provider flows are replaced, not extended. The Zitadel plan is dropped.
- D2. The Guard backend only VERIFIES account-service tokens (JWKS). It never issues its own session tokens in `account` mode.
- D3. Plans and limits come from the account service entitlements. Guard stores no prices, no subscriptions, no payment data.
- D4. Rollout is dual-mode behind `GUARD_AUTH_MODE=legacy|account` (default `legacy` until the final PR of this task, which flips the default to `account`). Legacy code is REMOVED later (Step 5), not in this task.
- D5. Local-first is unchanged: the CLI, agent and core must work fully offline and without any account. An account is required only for cloud/team features (policy sync, audit ingest, dashboard, org features).
- D6. Entitlements gate FEATURES only. An entitlement/auth/network failure must never change an allow/ask/deny decision and never weaken the hard-deny list, thresholds, or redaction.
- D7. DEFER the webhook receiver. No SQLite, no new datastore, no change to the account service contract. The receiver will be implemented per the durable-acceptance contract when Guard adopts a real production datastore.
- Account `planCatalog.policy.licenseTtlDays` is currently null. Offline-license issuance and persistence remain disabled; this PR adds only the verifier boundary.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Decision

- Use RFC 8628 device authorization as the default `algo login` flow. Retain an
  explicit loopback authorization-code + PKCE S256 option bound only to an
  ephemeral `127.0.0.1` callback.
- Pin `keyring` 3.6.3 with `windows-native`, `apple-native`, and
  `sync-secret-service`. It supports Windows Credential Manager, macOS
  Keychain, and Linux Secret Service while preserving the workspace's Rust
  1.75 MSRV. There is no plaintext fallback. This dependency choice requires
  owner confirmation.
- Treat absent or reused refresh-token rotation, provider reuse detection,
  subject changes, and invalid refresh grants as relogin conditions and delete
  the stored session.
- Keep the auth worker and crate independent from policy evaluation. All auth,
  vault, and network failures remain feature/login failures only.
- D7 remains in force: no webhook receiver or billing datastore is added.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Contract verification

Verified 2026-10-02 against the account repository without modifying it:

- `docs/INTEGRATION.md` requires discovery, public-client token exchange,
  scopes `openid profile email offline_access`, RFC 8628 polling behavior, and
  loopback PKCE on ephemeral `127.0.0.1` with exact state and nonce checks.
- `apps/api/src/oidc/clients.ts` registers `guard-cli` as a public native client
  with device-code, authorization-code, and rotated-refresh grants, token
  endpoint authentication `none`, and ES256 tokens.
- The account auth client verifies nonce only when one was supplied, matching
  nonce-free device authorization and nonce-required loopback authorization.

## Verification

- Agent `cargo fmt --check`: passed. Changed-package
  `cargo clippy --all-targets -- -D warnings`: passed.
- Agent `cargo test --workspace`: passed, 294 tests. Protocol tests cover
  pending/slow-down, token confusion, audience arrays, refresh rotation/reuse,
  invalid grants, expiry/cancel, loopback state/path, vault residue, and policy
  independence.
- `cargo deny check`: passed with the repository's existing warnings.
  `cargo audit`: passed with four existing allowed warnings (`paste`, two
  `lru` advisories, and yanked `yoke-derive`).
- Core format, Clippy, and tests passed. Backend format and 134 tests passed;
  current Rust 1.98 Clippy reports two pre-existing `needless_return` findings
  in `backend/src/policy.rs` and `backend/src/session.rs`, unchanged from the
  PR4 base and deliberately outside this PR.
- Web lint, 95 tests, and production build passed. Dashboard lint, 11 tests,
  and production build passed. `buf lint` and breaking comparison against
  `v0.0.1-alpha` passed. Changed-doc link check and Gitleaks diff scan passed.
- Windows release install → init → doctor → `algo uninstall` → installer
  uninstall passed in an isolated `ALGO_HOME`; no account-keyring marker or
  installed binary remained.
