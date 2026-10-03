# Step 3.5 PR6 — account default, docs, and manual E2E

## Summary

- Make account auth the backend and web default while retaining explicit
  `legacy` rollback flags.
- Document the complete identity/entitlement data flow, storage, failures,
  rollout configuration, rollback, privacy boundary, and deferred work.
- Add a cross-platform manual E2E runner for the real local account Compose
  stack, Guard backend/web, browser login, CLI device login, entitlement
  display, logout, and uninstall cleanup.
- Add an explicit extra-PEM-root option for local Compose. Certificate and
  hostname verification remain enabled; invalid configuration fails closed.

## Assumptions:

- D1. The central account service is the only identity provider. Guard's own
  signup/login/provider flows are replaced, not extended. The Zitadel plan is
  dropped.
- D2. The Guard backend only VERIFIES account-service tokens (JWKS). It never
  issues its own session tokens in `account` mode.
- D3. Plans and limits come from the account service entitlements. Guard stores
  no prices, no subscriptions, no payment data.
- D4. Rollout is dual-mode behind `GUARD_AUTH_MODE=legacy|account`. This final
  PR changes the default to `account`; legacy code is removed later in Step 5.
- D5. Local-first is unchanged: the CLI, agent and core work fully offline and
  without any account. An account is required only for cloud/team features.
- D6. Entitlements gate FEATURES only. An entitlement/auth/network failure
  never changes an allow/ask/deny decision and never weakens hard-deny,
  thresholds, or redaction.
- D7. DEFER the webhook receiver. No SQLite, no new datastore, no change to the
  account service contract. Guard will implement durable acceptance and
  long-lived dedupe when it adopts a production datastore.
- The account Compose stack uses a private local Caddy CA. The E2E exports its
  root to an OS temporary file and configures it as one additional trusted root;
  it never disables TLS validation.
- A read-only Guard service credential must already be provisioned in the local
  account database. The runner never creates, prints, or logs it.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Decision:

- Default backend and web authentication to `account`. Roll back only by
  setting both backend and web mode to `legacy` and restarting them together.
- Keep native local/offline commands account-optional; account login remains a
  cloud/team feature boundary.
- Trust a configured local PEM root in addition to public roots for the manual
  Compose E2E. Unreadable or invalid PEM stops startup/login; no insecure TLS
  switch is introduced.
- Keep CI hermetic and mock-based. The real browser, Docker, OS-keyring, and
  account-service run remains an explicitly manual E2E.
- D7 remains in force: account mode has no webhook route. The E2E asserts 404
  instead of pretending to test cache invalidation that cannot meet the durable
  receiver contract.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Contract verification

Verified 2026-10-03 against the read-only account repository:

- `README.md` documents Compose at `https://auth.localhost:8443`, generated
  ES256 JWKS, and browser trust for the local Caddy root.
- `infra/docker-compose.yml` registers local `guard-web`/`guard-cli` settings
  and runs the committed one-shot migration service before the API.
- `docs/INTEGRATION.md` defines discovery, PKCE/device flows, OS vault storage,
  exact issuer/audience checks, and ES256 JWKS verification.
- `docs/INTEGRATION_BILLING.md` defines the entitlement endpoint, 60-second
  cache control, five-minute stale-if-error window, and the durable webhook
  receiver requirement. The receiver remains deferred under D7.
- The account docs do not publish a Guard billing URL, do not say whether a
  product delivery endpoint is optional, and do not describe provisioning an
  initial local admin/service key. These remain deployment/manual prerequisites,
  not invented Guard APIs.
- On 2026-10-03, reqwest 0.12.28 documentation was checked for
  [`Certificate::from_pem`](https://docs.rs/reqwest/0.12.28/reqwest/tls/struct.Certificate.html#method.from_pem)
  and
  [`ClientBuilder::add_root_certificate`](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html#method.add_root_certificate);
  the workspace lock resolves 0.12.28.

## Verification

- Backend: formatting, 135 tests, and Clippy with `-D warnings` passed.
- Agent: formatting and all 294 workspace tests passed; focused Clippy for the
  changed account-auth and CLI crates passed. Full-workspace Clippy remains
  blocked by six pre-existing Windows-only unused/dead-code diagnostics in
  `agent/crates/daemon/src/main.rs`, outside this PR.
- Core: formatting, tests, and Clippy passed.
- Web: lint, 95 tests, the default account build, and the explicit legacy
  rollback build passed. Dashboard lint, 11 tests, and build passed.
- Proto: `buf lint` and breaking comparison against `v0.0.1-alpha` passed;
  proto is unchanged. Cargo-deny passed with the repository's existing warning
  set; no dependency or lockfile changed in this PR.
- The real account/browser/keyring E2E remains manual by design and requires a
  locally provisioned service key; CI coverage is mock-based and hermetic.
- This PR needs human CODEOWNERS review for auth, TLS trust configuration,
  logout, and uninstall behavior.
