# backend (Phase 0 — no product code yet)

> Part of **algorithco guard** (CLI algo). Monorepo: `algorithcoguard/algorithco-guard`.
> Real code lives in the monorepo subdirectory `backend/` starting its build phase. This dir is a scaffold stub.

Scope: team cloud API (Phase 3 only) — auth (device flow), orgs, signed policy sync,
opt-in audit ingest, GitHub App. Local-first: useful with no cloud account.

## Phase 1+ brief

Contracts-first: proto tag, pin exact,
generate at build. Fail-safe (ask, never allow), latency budgets in CI, no secrets.

## Now

Product code lives here (in-memory MVP).

## Authentication modes

`GUARD_AUTH_MODE=legacy|account` selects authentication. The default remains
`legacy` during the staged account-service rollout.

In `account` mode, the backend discovers the central Algorithco account
service from `GUARD_ACCOUNT_ISSUER`, fetches its ES256 JWKS, and verifies
account access tokens locally. It never mints backend sessions and the legacy
signup/provider routes are not mounted. Tokens must have `typ=at+jwt`, a
nonempty `client_id` and `scope`, and no ID-token markers. `sub` is the sole
ownership identity; email is display/contact data only.

The temporary audience bridge is an exact allow-list with no code default.
`GUARD_ACCOUNT_AUDIENCES` is required in account mode and rejects empty,
whitespace-bearing, wildcard, and duplicate entries. The current account
service emits client-ID audiences, so the deployment value is
`guard-web,guard-cli`. This changes to one dedicated Guard API audience when
the account service adds it.

JWKS responses are size-limited and cached using bounded `Cache-Control`
`max-age` (at most ten minutes) plus `stale-if-error` (at most five minutes).
An unknown `kid` triggers one rate-limited refresh. A cold-cache outage or
invalid token fails closed with 401; a bounded stale key may be used during a
JWKS outage.

| Var | Purpose |
|---|---|
| `GUARD_AUTH_MODE` | `legacy` or `account`; default `legacy` until the final rollout PR |
| `GUARD_ACCOUNT_ISSUER` | required in account mode; exact HTTPS issuer (loopback HTTP allowed for local tests) |
| `GUARD_ACCOUNT_AUDIENCES` | required exact-match comma list; current bridge value `guard-web,guard-cli` |
| `GUARD_ACCOUNT_CLOCK_SKEW_SECS` | expiry/not-before/future-issued-at leeway, `0..300`; default `5` |

The JWKS URI is intentionally not configured separately: it is read from
`/.well-known/openid-configuration`, whose returned issuer must exactly match
`GUARD_ACCOUNT_ISSUER`.

## Legacy OAuth login (private MVP, backend only)

Email/password accounts (`POST /v1/auth/signup|login`, argon2id-hashed,
in-memory users) plus GitHub device flow + Google OIDC (installed-app
loopback + PKCE, JWKS-verified ID tokens). Successful logins mint short-lived
(1h) HS256 backend session tokens; provider refresh tokens are returned to
the caller (BYOK) — the backend keeps no refresh store. API console: `GET /`
or `/ui`. There is no terminal device-code login; `algo login` uses email or
provider sign-in via the web flow.

Env:

| Var | Purpose |
|---|---|
| `ALGO_BACKEND_ADDR` | bind addr (default `127.0.0.1:8080`) |
| `ALGO_CORS_ORIGINS` | extra CORS origins for direct browser fetches (comma-separated `scheme://host:port`, no trailing slash; default allows `http://127.0.0.1:3007` + `http://localhost:3007` for the web dev server); invalid entries skipped fail-closed |
| `ALGO_GITHUB_CLIENT_ID` | GitHub App/OAuth App client ID (device flow must be enabled in app settings); unset → GitHub routes 503 |
| `ALGO_GOOGLE_CLIENT_ID` | Google Cloud "Desktop app" OAuth client ID; unset → Google routes 503 |
| `ALGO_GOOGLE_CLIENT_SECRET` | optional; without it the backend acts as a public client (PKCE only) |
| `ALGO_SESSION_JWT_SECRET` | required only in legacy mode; HS256 secret (minimum 32 bytes); missing or shorter values prevent startup |
| `ALGO_GITHUB_DEVICE_CODE_URL` / `ALGO_GITHUB_ACCESS_TOKEN_URL` / `ALGO_GITHUB_API_BASE` | override for staging / GitHub Enterprise Server |
| `ALGO_GOOGLE_AUTH_URL` / `ALGO_GOOGLE_TOKEN_URL` / `ALGO_GOOGLE_USERINFO_URL` / `ALGO_GOOGLE_JWKS_URL` | override for staging / offline dev |
| `ALGO_POLICY_SIGNING_SEED_HEX` | required 64-hex policy signing seed; missing or malformed values prevent startup |

Routes: `POST /v1/auth/signup|login` (email, open; login is oracle-free 401),
`POST /v1/auth/github/device|poll|validate`,
`POST /v1/auth/google/url|callback|verify`. All fail closed (400 invalid,
401 unauthorized, 409 duplicate, 503 not-configured, 502 provider-down,
429 slow-down).

Dashboard shims (Phase 2 v1, dashboard plan proposed): `GET /v1/audit`
(paginated newest-first list: `limit/cursor/decision/tool_kind/from/to/org_id`;
unknown filters match nothing), `GET /v1/audit/stream` (bounded-burst SSE tail,
`retry: 5000`, `Last-Event-ID` resume), `GET /v1/stats` (now honors
`from/to` + `granularity=day|hour`, `limit`, `top_n`; returns
`buckets/per_user/per_project/truncated`), `GET /v1/orgs` + `GET /v1/orgs/:id`
(owner-scoped, no-oracle 404). All in-memory: restart wipes audit + orgs.
Default CORS allows loopback `:3007` (web) and `:5173` (dashboard dev);
extend via `ALGO_CORS_ORIGINS`.

Billing enforcement (`plans.rs|subscriptions.rs`): subscriptions bind to the
creator's caller identity (owner); org_id alone never spends a plan —
non-owners resolve exactly like strangers (free shape, 402, 404; no
existence oracle). Owner is server-side only (never serialized). One live
subscription per org (409 on double-create, dead rows pruned). Changes are
atomic (upgrades immediate, downgrades at period end, combined tier+seats in
one call). Absolute seat cap 100k. Multi-user org membership is future work
(single-owner MVP).

Security properties (verified against RFC 9700 BCP + OIDC Core, 2026-09-24):

- Google `redirect_uri` is SERVER-PINNED (`ALGO_GOOGLE_REDIRECT_URI`,
  default exact loopback `http://127.0.0.1:51004/oauth2redirect`); any other
  caller-supplied value is rejected (login-CSRF prevention).
- Per-transaction PKCE S256 + OIDC `nonce` (server-side, one-shot `state`);
  `max_age=0` forces fresh user authentication with `auth_time` verified.
- ID tokens: RS256 pinned, `kid` required + matched, `aud`/`iss`/`exp`
  enforced (zero leeway), `iat` ≤15min old, `auth_time` inside the flow
  window, `at_hash` checked when the access token is known, `sub` ≤255 chars.
  `jku`/`x5u` never trusted. Identity is `sub` only, never `email`.
- `POST /v1/auth/google/verify` requires `expected_nonce` (replay binding).
- Provider JSON bodies size-capped (256KB; JWKS 64KB, 1h cache).
- Sessions: HS256, 1h TTL, fixed `iss=algo-backend`, zero leeway.
- No tokens, codes, or PII in logs (rate-limit keys are hashes); no cookies
  (CSRF N/A); no redirects (open-redirect N/A); all outbound provider URLs
  are config-controlled (SSRF N/A).
- `cargo audit` / `cargo deny check` clean (`rsa` dev-only advisory
  RUSTSEC-2023-0071 documented in `deny.toml` + `.cargo/audit.toml`).
