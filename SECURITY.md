# Security policy

## Support status

Algorithco Guard is pre-production. No released version currently carries a
general-availability security-support commitment. Release artifacts must pass
the repository's formatting, lint, test, dependency, secret, provenance, and
signature gates; auth, policy-signature, deny-list, redaction, install, and
uninstall changes also require human CODEOWNERS review.

## Trust boundaries

Core command policy evaluation is local-first and does not require an account
or network. I/O, parse, timeout, and provider failures on a decision path must
produce `ask`, never `allow`; entitlements cannot weaken hard-deny rules,
thresholds, or redaction.

Account mode is the default for cloud/team APIs. The central Algorithco account
service is the sole identity and entitlement authority:

- The web and CLI use public-client OIDC flows with PKCE or RFC 8628. They do
  not contain a client secret.
- The backend accepts only ES256 access tokens verified against discovered
  JWKS, exact issuer and configured audiences. ID-token markers are rejected.
- Browser tokens are memory-only. Native credentials are stored in the OS
  credential vault with no plaintext fallback. Tokens and service credentials
  must not appear in logs.
- The backend keeps bounded JWKS and entitlement caches in memory. It persists
  no prices, subscriptions, payment details, or webhook events.
- The product webhook receiver is deliberately absent until Guard has durable
  storage capable of meeting the account service's acceptance and dedupe
  contract. Entitlement staleness is bounded by cache revalidation and
  `validUntil` outage handling.

See [`docs/ACCOUNT_INTEGRATION.md`](docs/ACCOUNT_INTEGRATION.md) for the exact
data flow, failure behavior, rollout configuration, rollback, and known gaps.

## Reporting a vulnerability

Do not disclose an unpatched vulnerability in a public issue. Use the
repository host's private vulnerability-reporting channel when it is enabled,
or contact the repository owner through an established private channel. A
verified public security address and response SLA remain required before
general availability.
