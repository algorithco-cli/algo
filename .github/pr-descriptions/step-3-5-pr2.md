## Summary

Use central account entitlements in account mode through a typed, bounded,
claim-aware cache and a caller-only refresh endpoint. Do not expose a webhook
receiver until Guard has durable production storage.

Task ID: Step 3.5 / PR2

This PR needs human CODEOWNERS review because it changes authentication-adjacent
entitlement enforcement. Agent review is not sufficient.

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

- D7. DEFER the webhook receiver. No SQLite, no new datastore, no change to the account service contract. The account service remains authoritative; an unconfigured invalidation hint can only delay a downgrade/cancellation until the bounded cache TTL. Upgrades can be fetched immediately through the authenticated refresh endpoint.
- The account receiver contract says: “Deduplicate `event_id` permanently enough to cover retries. Return 2xx only after durable acceptance; process asynchronously.” Account mode therefore registers no webhook route.
- Successful entitlement freshness is capped at the account contract's 60-second `max-age`, which is stricter than the owner-approved configurable default of 300 seconds. Outage fallback is separate and lasts only through `validUntil`.
- The cache is in-memory, limited to 1,024 subjects, and singleflight-coalesced per subject. No billing/subscription state is persisted.
- `POST /v1/entitlements/refresh` derives the only subject from the verified token and accepts no subject field.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.

## Contract verification

Verified 2026-10-02 against the account repository:

- `docs/INTEGRATION_BILLING.md`: compact claim `{p,plan,exp?}`, service-key endpoint, 60-second cache plus five-minute stale-if-error guidance, `valid_until` outage fallback, and durable webhook requirements.
- `apps/api/src/http/billing-routes.ts`: `GET /v1/users/:sub/entitlements`, Bearer `alg_sk_...`, response `ETag`, and `Cache-Control: private, max-age=60, stale-if-error=300`.
- `apps/api/src/billing/entitlement-service.ts` and `billing/entitlements.ts`: camelCase response shape and effective entitlement fields.
- `reqwest` 0.12.28 request timeout, bearer auth, conditional headers, response headers/status, and bounded chunk reads: https://docs.rs/reqwest/0.12.28/reqwest/

The account implementation emits `ETag` but does not currently inspect
`If-None-Match`; Guard sends it and supports a future standards-compliant 304.
The docs do not state whether outbound delivery endpoints are optional per
product. No endpoint is configured or invented here.

## Verification

- `cargo fmt --check`: passed.
- `cargo clippy -- -D warnings`: passed.
- `cargo test`: passed, 133 tests.
- `cargo deny check`: passed; existing duplicate, unmatched-license, and
  allowed yanked-crate warnings remain.
- `cargo audit`: passed with the existing allowed yanked `yoke-derive`
  warning.
- Web: lint passed, 82 tests passed, production build passed.
- Dashboard: lint passed, 11 tests passed, production build passed.
- `buf lint proto`: passed.
- `buf breaking proto --against '.git#branch=main,subdir=proto'`: passed.
- `gitleaks git --redact --no-banner`: passed.
- Markdown link checks for the changed integration docs: passed.
