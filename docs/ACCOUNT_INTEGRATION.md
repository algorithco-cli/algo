# Algorithco account integration

Guard defaults to central account mode. The backend uses
`GUARD_AUTH_MODE=legacy|account`; the web build uses the matching
`VITE_GUARD_AUTH_MODE`. Both default to `account`. Account mode verifies
central ES256 access tokens locally and uses the account service as the only
source of identity and entitlement truth. Local/offline Guard behavior does
not require an account.

## Configuration

| Component | Variable | Behavior |
|---|---|---|
| Backend | `GUARD_AUTH_MODE` | Optional; defaults to `account`. Set exactly `legacy` for rollback. |
| Backend | `GUARD_ACCOUNT_ISSUER` | Required in account mode; exact HTTPS issuer. Loopback HTTP is test-only. |
| Backend | `GUARD_ACCOUNT_AUDIENCES` | Required exact allow-list. Temporary value: `guard-web,guard-cli`; empty, whitespace, wildcard and duplicate values fail startup. |
| Backend | `GUARD_ACCOUNT_CLOCK_SKEW_SECS` | Optional `0..300`; default `5`. |
| Backend | `GUARD_ACCOUNT_CA_CERT` | Optional path to one extra PEM root for local Compose. It does not disable TLS or hostname verification. |
| Backend | `GUARD_ACCOUNT_SERVICE_KEY` | Required read-only, Guard-scoped `alg_sk_...` service credential. Supply through the secret manager. |
| Backend | `ENTITLEMENT_CACHE_MAX_TTL_SECS` | Optional `1..300`; default `300`, but the account contract's 60-second `max-age` wins. |
| Backend | `ENTITLEMENT_REFRESH_MIN_INTERVAL_SECS` | Optional `1..3600`; default `30`. |
| Web | `VITE_GUARD_AUTH_MODE` | Optional build value; defaults to `account`. Set exactly `legacy` for rollback. |
| Web | `VITE_ACCOUNT_ISSUER` | Required in account mode. |
| Web | `VITE_ACCOUNT_REDIRECT_URI` | Required exact `guard-web` redirect URI. |
| Web | `VITE_ACCOUNT_BILLING_URL` | Required account-owned billing page URL; currently a deployment TODO. |
| Web | `VITE_BACKEND_URL` | Guard backend origin for static preview/deployment; Vite development can use its `/api` proxy. |
| CLI | `ALGO_ACCOUNT_ISSUER` | Optional issuer override; production default is `https://auth.algorithco.com`. |
| CLI | `ALGO_ACCOUNT_CLOCK_SKEW_SECS` | Optional `0..60`; default `5`. |
| CLI | `ALGO_ACCOUNT_CA_CERT` | Optional path to one extra PEM root for local Compose only. |

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
`ALGO_ACCOUNT_CA_CERT` may name one additional PEM root only for local
development with the account Compose stack. It adds that root to normal TLS
verification; it does not disable certificate or hostname checks.

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

## Data flow, storage, and failure behavior

- Web and CLI send OIDC requests directly to the configured account issuer.
  The Guard backend receives only the bearer access token on authenticated
  cloud/team API calls. It verifies the token locally from discovered JWKS.
- The backend sends the opaque account `sub` in the documented entitlement
  path and its product-scoped service credential in the Authorization header.
  It does not send command text, policy input, local audit contents, or payment
  data to the account service.
- Browser tokens live only in process memory. Native refresh/access tokens live
  only in the operating-system credential vault. Guard stores the non-secret
  CLI marker described above and in-memory entitlement/JWKS caches; it does not
  persist prices, subscriptions, card data, or billing webhook events.
- Invalid or unverifiable authentication fails closed as unauthenticated. A
  cold entitlement failure uses the free tier; a warm failure may use the last
  known good response only through its own `validUntil`. Neither path can alter
  an allow/ask/deny policy decision.
- Core policy evaluation, the daemon, local audit inspection, and offline TUI
  operation remain usable without an account or network. Cloud/team operations
  require a valid account session.

## Rollback to legacy mode

Rollback does not delete the account integration. Set
`GUARD_AUTH_MODE=legacy` on the backend and rebuild the web application with
`VITE_GUARD_AUTH_MODE=legacy`, then restart both components together. Legacy
mode requires its existing `ALGO_SESSION_JWT_SECRET` and exposes the former
Guard-local login, plan, and subscription routes. Do not mix an account-mode
web build with a legacy backend or the reverse. CLI local/offline behavior is
unchanged; `algo logout` removes any account credential before or after a
rollback. Returning to account mode requires all account variables in the
table above, including the explicit audience and service credential.

## Verification: CI mocks versus manual local E2E

CI is hermetic: backend tests use local mock discovery/JWKS/entitlement servers,
web tests mock browser OIDC and backend calls, and CLI tests use a mock RFC 8628
server and test credential store. CI does not contact the account service or
exercise a real platform keyring.

The manual cross-platform runner is
[`scripts/account-integration-e2e.mjs`](../scripts/account-integration-e2e.mjs).
It is Windows-compatible and starts Docker Compose with the account checkout as
its working directory. It writes a generated local ES256 key and Compose
override only under the operating-system temporary directory, then proves that
the account checkout remains clean. The account stack's own Compose definition
runs its committed one-shot migration service against its Docker database; the
runner never invokes account migration or seed commands directly and never
writes the account repository.

Prerequisites are Docker, Node.js, Rust, browser trust for the exported local
Caddy root, and a `GUARD_ACCOUNT_SERVICE_KEY` already provisioned in the same
local account database. The account contract deliberately shows this key only
once; pass it through the environment, never the command line. Then run:

```powershell
$env:GUARD_ACCOUNT_SERVICE_KEY = "<local alg_sk_ credential>"
node .\scripts\account-integration-e2e.mjs
Remove-Item Env:GUARD_ACCOUNT_SERVICE_KEY
```

The runner automates service startup/readiness, account-mode configuration,
the deferred-webhook 404 assertion, isolated CLI init, marker checks, logout,
uninstall cleanup, and account-worktree verification. Web login, the displayed
entitlement, web logout, and two device-flow approvals are human-confirmed
because the real account pages and OS/browser credential boundaries must remain
interactive. Use `--no-build` only after building the backend and CLI; use
`--keep-services` to leave services running. Logs and temporary keys are
deleted on exit and tokens are never printed.

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
- Document the supported bootstrap for the first local account administrator
  and Guard service credential. The account repository exposes admin-only
  service-key creation but its integration docs do not describe initial local
  admin provisioning.
- Configure real production client redirect URIs.
- Publish and configure the account billing/checkout URL and its Guard return
  URL; the current account integration docs do not define one.
- Decide prices, numeric limits, and offline-license TTL in the account
  service; Guard does not invent them.
- Account plan configuration currently has no offline-license TTL. The native
  client therefore exposes only a verification trait; offline-license issuance
  and persistence remain disabled until a TTL is configured.
- Choose a legacy-auth removal date after the staged rollout.
