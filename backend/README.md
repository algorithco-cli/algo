# backend (Phase 0 — no product code yet)

> Part of **algorithco guard** (CLI algo). Monorepo: `algorithcoguard/algorithco-guard` (see docs/github-org-plan.md).
> Real code lives in the monorepo subdirectory `backend/` starting its build phase. This dir is a scaffold stub.

Scope: team cloud API (Phase 3 only) — auth (device flow), orgs, signed policy sync,
opt-in audit ingest, GitHub App. Local-first: useful with no cloud account.

## Phase 1+ brief

See plans/00-index-build-order.md for build order. Contracts-first: proto tag, pin exact,
generate at build. Fail-safe (ask, never allow), latency budgets in CI, no secrets.

## Now (Phase 0)

Empty except .gitkeep + this stub. Do NOT add product code until docs/exit-gate-P0.md is signed.

## OAuth login (private MVP, backend only)

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
| `ALGO_SESSION_JWT_SECRET` | HS256 secret (min 16 chars); unset → ephemeral per-boot dev secret |
| `ALGO_GITHUB_DEVICE_CODE_URL` / `ALGO_GITHUB_ACCESS_TOKEN_URL` / `ALGO_GITHUB_API_BASE` | override for staging / GitHub Enterprise Server |
| `ALGO_GOOGLE_AUTH_URL` / `ALGO_GOOGLE_TOKEN_URL` / `ALGO_GOOGLE_USERINFO_URL` / `ALGO_GOOGLE_JWKS_URL` | override for staging / offline dev |
| `ALGO_POLICY_SIGNING_SEED_HEX` | 64-hex policy signing seed; unset → deterministic test key + warning |

Routes: `POST /v1/auth/signup|login` (email, open; login is oracle-free 401),
`POST /v1/auth/github/device|poll|validate`,
`POST /v1/auth/google/url|callback|verify`. All fail closed (400 invalid,
401 unauthorized, 409 duplicate, 503 not-configured, 502 provider-down,
429 slow-down).

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
