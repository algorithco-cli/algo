# Algorithco account integration

Guard's staged account integration uses `GUARD_AUTH_MODE=legacy|account`.
Account mode verifies central ES256 access tokens locally and uses the account
service as the only source of identity and entitlement truth. Local/offline
Guard behavior does not require an account.

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

### Human TODOs

- Provision and rotate the read-only Guard product service credential.
- Configure real production client redirect URIs.
- Decide prices, numeric limits, and offline-license TTL in the account
  service; Guard does not invent them.
- Choose a legacy-auth removal date after the staged rollout.
