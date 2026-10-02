## Summary

Add dual-mode backend authentication and fail-closed local verification of
Algorithco account access tokens through discovery and a bounded JWKS cache.

Task ID: Step 3.5 / PR1

This PR needs human CODEOWNERS review because it changes authentication and
signature verification. Agent review is not sufficient.

## Assumptions

- D1. The central account service is the only identity provider. Guard's own signup/login/provider flows are replaced, not extended. The Zitadel plan is dropped.
- D2. The Guard backend only VERIFIES account-service tokens (JWKS). It never issues its own session tokens in `account` mode.
- D3. Plans and limits come from the account service entitlements. Guard stores no prices, no subscriptions, no payment data.
- D4. Rollout is dual-mode behind `GUARD_AUTH_MODE=legacy|account` (default `legacy` until the final PR of this task, which flips the default to `account`). Legacy code is REMOVED later (Step 5), not in this task.
- D5. Local-first is unchanged: the CLI, agent and core must work fully offline and without any account. An account is required only for cloud/team features (policy sync, audit ingest, dashboard, org features).
- D6. Entitlements gate FEATURES only. An entitlement/auth/network failure must never change an allow/ask/deny decision and never weaken the hard-deny list, thresholds, or redaction.
- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.
- The account service's pinned `oidc-provider` 9.12.2 token formatter was exercised locally on 2026-10-02 without logging tokens. Access tokens had `typ=at+jwt`, `client_id`, and `scope`; ID tokens did not.

## Decision

- Temporary: access-token `aud` equals the client ID in the account service. Follow-up: the account service will issue access tokens with a dedicated Guard API audience (resource indicator). When that ships, `GUARD_ACCOUNT_AUDIENCES` is changed to that single value and client-ID audiences are removed.
- `GUARD_ACCOUNT_AUDIENCES` is required in account mode and is parsed as an exact-match allow-list. Empty, wildcard, whitespace-bearing, and duplicate entries fail startup.
- Access-token identity is the nonempty opaque `sub`. Email is never used for authorization.

## Verification references

Verified on 2026-10-02 against the exact locked versions:

- `jsonwebtoken` 9.3.1 validation, required registered claims, audience membership, leeway, and optional `nbf`: https://docs.rs/jsonwebtoken/9.3.1/jsonwebtoken/struct.Validation.html
- `jsonwebtoken` 9.3.1 unverified header parsing and EC JWK components: https://docs.rs/jsonwebtoken/9.3.1/jsonwebtoken/fn.decode_header.html and https://docs.rs/jsonwebtoken/9.3.1/jsonwebtoken/struct.DecodingKey.html
- `reqwest` 0.12.28 request timeout and chunked response reads: https://docs.rs/reqwest/0.12.28/reqwest/struct.RequestBuilder.html and https://docs.rs/reqwest/0.12.28/reqwest/struct.Response.html
- `http` 1.5.0 `HeaderMap::get_all` duplicate-header handling: https://docs.rs/http/1.5.0/http/header/struct.HeaderMap.html
- `p256` 0.13.2 ephemeral signing keys and PKCS#8 encoding (tests only): https://docs.rs/p256/0.13.2/p256/
- Pinned `oidc-provider` 9.12.2 JWT access-token format (`typ=at+jwt`, `client_id`, `scope`): https://github.com/panva/node-oidc-provider/blob/v9.12.2/lib/models/formats/jwt.js
- RFC 9068 access-token JWT `typ=at+jwt`: https://www.rfc-editor.org/rfc/rfc9068.html

## Verification

- `cargo fmt --check`: passed.
- `cargo clippy -- -D warnings`: passed.
- `cargo test`: passed, 121 tests.
- `cargo deny check`: passed; existing duplicate/unmatched-license/yanked warnings remain non-blocking.
- `cargo audit`: passed; the existing allowed yanked `yoke-derive` warning remains.
- `gitleaks git --redact --no-banner`: passed, no leaks found.
- `buf lint proto`: passed.
- `buf breaking proto --against '.git#branch=main,subdir=proto'`: passed.
- Web: lint passed, 82 tests passed, production build passed.
- Dashboard: lint passed, 11 tests passed, production build passed.
- `cargo clippy --all-targets -- -D warnings` is not the repository's listed gate and reports two pre-existing test-only `needless_return` warnings in `policy.rs` and `session.rs`. Their fixes remain isolated on `wip/preexisting-policy-session-20261002` per owner instruction and are not included here.
