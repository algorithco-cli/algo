# Algorithco account integration

Guard's staged account integration uses `GUARD_AUTH_MODE=legacy|account`.
Account mode verifies central ES256 access tokens locally and uses the account
service as the only source of identity and entitlement truth. Local/offline
Guard behavior does not require an account.

## Native CLI sign-in (`guard-cli`)

`algo login` uses RFC 8628 device authorization by default. It discovers the
device and token endpoints from the configured issuer, requests exactly
`openid profile email offline_access`, displays the account-provided
verification URL and user code, respects `interval`, adds five seconds after
each `slow_down`, and stops on cancellation or expiry. `algo login --flow
loopback` is also available: it binds only an ephemeral `127.0.0.1` port at
`/callback`, uses authorization code + PKCE S256 with random state and nonce,
and requires an exact state match before exchanging the code. Neither flow
uses a client secret.

`ALGO_ACCOUNT_ISSUER` optionally overrides the production issuer for a local
account-service run, and `ALGO_ACCOUNT_CLOCK_SKEW_SECS` configures token clock
skew from 0 through 60 seconds (default 5). Non-loopback issuers and all
discovered account endpoints must use HTTPS and remain on the issuer origin.

The CLI accepts only ES256 tokens verified against discovered JWKS. It
requires issuer, `guard-cli` audience, `exp`, `iat`, and `sub`; applies the
configured clock-skew bound to `iat` and optional `nbf`; and requires `azp` to
be `guard-cli` for a multi-valued audience. Access tokens must have
`typ=at+jwt`, `client_id=guard-cli`, and a non-empty `scope`, and must not have
ID-token markers (`nonce`, `at_hash`, or `auth_time`). ID tokens are rejected
if they carry access-token markers, and loopback ID tokens must carry the
attempt nonce. Tokens are never logged.

Access and rotated refresh tokens are stored as one opaque record in the
operating-system credential vault: Windows Credential Manager, macOS Keychain,
or Linux Secret Service. There is no plaintext fallback. A non-secret marker
under `~/.algo` lets uninstall fail closed if a credential should exist but
the vault cannot be reached; it contains no token, subject, or account data.
The client refreshes expiring access tokens transparently. An
`invalid_grant`/`invalid_token`, subject change, or missing/reused replacement
refresh token deletes the stored session and requires `algo login` again.
`algo logout`, `algo uninstall`, and the Windows uninstall script remove the
vault record before reporting success.

Local policy evaluation, the daemon, audit viewing, and offline TUI mode do
not require login. The account-auth crate has no dependency on the policy
engine, so discovery, login, refresh, vault, and network failures cannot
change an allow/ask/deny result.

## Web sign-in (`guard-web`)

Account-mode web builds use the discovered OIDC authorization and token
endpoints with authorization code + PKCE S256. Each attempt creates random
`state`, `nonce`, and verifier values; the one-shot login transaction is kept
in `sessionStorage`, contains no token, and expires after ten minutes. The
callback compares state, exchanges the code as the public `guard-web` client,
and verifies the ES256 ID-token signature, issuer, `guard-web` audience,
expiry, issued-at time, subject, authorized party (for multiple audiences),
and nonce against discovered JWKS before accepting the access token.

Access, ID, and refresh tokens are process-memory only. They are never written
to local or session storage. A reload therefore starts a top-level redirect to
the account service; its SSO session can make that round trip seamless. Logout
first clears every local token and pending transaction, then navigates to the
discovered end-session endpoint. No iframe, hidden form, client secret, or
third-party script is used.

Required build variables are `VITE_GUARD_AUTH_MODE=account`,
`VITE_ACCOUNT_ISSUER`, `VITE_ACCOUNT_REDIRECT_URI`, and
`VITE_ACCOUNT_BILLING_URL`. Redirect matching is exact. The account repository
currently shows `https://guard.algorithco.com/auth/callback` for production;
local development must register its exact loopback URL separately. The billing
URL is deliberately configuration, because the account integration docs do
not publish a billing-page or checkout endpoint. Configure its return URL as
`/billing?account_return=1`; Guard then calls the caller-only entitlement
refresh endpoint after the redirect. Pricing and billing display the effective
backend entitlement and do not expose Guard's legacy price or card UI in
account mode.

## Entitlement resolution

1. Use a compact `guard` entry from the verified access token's
   `entitlements` claim when one is present and both its expiry and the local
   60-second freshness bound remain valid.
2. Otherwise call `GET /v1/users/{sub}/entitlements` with a read-only,
   Guard-product-scoped service credential.
3. Cache successful responses in memory using `ETag` and `Cache-Control`.
   `If-None-Match` is sent during revalidation. The account contract specifies
   `max-age=60`; this is stricter than the owner-approved configurable default
   of 300 seconds and is therefore the effective hard freshness ceiling.
4. On timeout or API failure, use the last-known-good entitlement only until
   its own `validUntil`; otherwise use Guard's free tier. Local features remain
   available and policy decisions remain independent of entitlement I/O.

The single plan-to-feature mapping is
[`backend/guard-plan-features.json`](../backend/guard-plan-features.json).
Entitlement `limits` are passed through from the account response. Guard does
not infer prices, seats, retention, or other numeric product limits.

`POST /v1/entitlements/refresh` is authenticated and refreshes only the
verified caller's `sub`; it has no subject parameter. The default per-subject
minimum interval is 30 seconds. Upgrades can therefore become visible as soon
as the user returns from account billing and requests a refresh. Downgrades
and cancellations become visible within the normal cache TTL.

## Known gaps / deferred

### Product webhook receiver

Owner decision D7 defers the webhook receiver. Guard has no durable production
datastore, persists no billing/subscription data, and its current cache is an
in-memory, loss-tolerant optimization. The account entitlement API remains
authoritative. Account documentation does not say whether a delivery endpoint
is optional per product, so deployment must not configure a Guard delivery
endpoint until this receiver exists.

The account contract requires receivers to:

> “Deduplicate `event_id` permanently enough to cover retries. Return 2xx only
> after durable acceptance; process asynchronously.”

An in-memory receiver cannot satisfy that contract and is intentionally not
registered in account mode. When Guard adopts a shared production datastore,
the receiver design is: verify the raw-body HMAC and five-minute timestamp;
support a documented secret-rotation procedure; insert into a durable inbox
with a unique `event_id` before returning 2xx; process cache invalidation in an
asynchronous worker; and retain deduplication records for at least the account
service retry window.

### Temporary access-token audience

Temporary: access-token `aud` equals the client ID in the account service.
Follow-up: the account service will issue access tokens with a dedicated Guard
API audience (resource indicator). When that ships,
`GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID
audiences are removed.

### Organization membership and roles

The account service internally uses organization memberships and assigned
billing seats when computing effective entitlements. It does not currently
publish organization membership or role claims in Guard access tokens, and it
does not expose a product-facing membership/role lookup endpoint. The public
entitlement response may identify an organization as the winning entitlement
owner, but that is not an authorization-role contract.

Until the account service exposes such a contract, Guard keeps its existing
local organization registry and maps its sole owner directly to the verified
account `sub`. Every `/v1/orgs*`, `/v1/policy*`, `/v1/audit*`, and `/v1/stats`
operation performs this owner check. Unknown organizations and other owners'
organizations use the same not-found response so the API does not become an
existence oracle. Multi-member roles, membership synchronization, and
account-managed team authorization remain unavailable; Guard does not infer
or invent them.

### Human TODOs

- Provision and rotate the read-only Guard product service credential.
- Configure real production client redirect URIs.
- Publish and configure the account billing/checkout URL and its Guard return
  URL; the current account integration docs do not define one.
- Decide prices, numeric limits, and offline-license TTL in the account
  service; Guard does not invent them.
- Account plan configuration currently has no offline-license TTL. The native
  client therefore exposes only a verification trait; offline-license issuance
  and persistence remain disabled until a TTL is configured.
- Choose a legacy-auth removal date after the staged rollout.
