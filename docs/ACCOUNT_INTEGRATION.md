# Algorithco account integration

Guard's staged account integration uses `GUARD_AUTH_MODE=legacy|account`.
Account mode verifies central ES256 access tokens locally and uses the account
service as the only source of identity and entitlement truth. Local/offline
Guard behavior does not require an account.

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
- Choose a legacy-auth removal date after the staged rollout.
