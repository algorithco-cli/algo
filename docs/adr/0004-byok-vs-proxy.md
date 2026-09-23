# ADR-0004: Jev BYOK-only (no proxy mode)

## Status

accepted — 2026-09-23 (D-A / D-01; owner @algorithcoguard/backend; team decision: BYOK only, proxy will not be built. Human sign-off block below stays blank pending signature.)

## Context

L3 judgments call Jev over the network (see `docs/threat-model-v0.md` B2). The choice shapes
P3 scope, privacy posture, and cost: who holds the API key, what leaves the machine, and who
pays. Source: `plans/90-crosscutting-gates-ux-decisions.md` §3 (D-01) and Phase 0 draft D-A.
Tracking: P0-JEV-1 / P0-JEV-2 (API/SDK dossier + measurement); no L3 product path may ship
without this ADR merged.

## Decision

**Bring your own Jev API key (BYOK) is the ONLY supported Jev mode** — this ADR's decision,
confirmed 2026-09-23. algorithco guard never proxies or shares its own API key: each end
user — or their org, via an Algorithco subscription — supplies and connects their own
TypeSafe API key. **No embedded key in distributed binaries** (see MCA §2.4 Q2 — keys
confidential, no sharing; §2.3(b)/(g) uncapped risk). **Proxy mode through our servers is
explicitly out of scope and will not be built** (not deferred — decided). Builds, docs,
and UX assume the user supplies their own Jev key. Redact-before-network applies;
`local-only` (default) sends nothing except via the explicit Jev path with opt-in consent;
`algo log --show-egress` shows what left the machine.

## Alternatives

 - **Proxy-first (backend holds a shared key, clients call backend)**: **rejected 2026-09-23 — proxy mode will not be built** (was "blocked until legal review"). It would pull auth, billing, rate-limiting, abuse handling, and **key custody** (MCA §2.4 confidential) into scope and concentrate disclosure + uncapped indemnity risk (§§12.3, 13.2). Prior "revisit after legal review" note is superseded by this decision.
- **BYOK-only forever (no proxy trait)**: **chosen 2026-09-23** (was "rejected" — reversed by this decision). Team/org flows are served by org-supplied keys via Algorithco subscription, not by a proxy. The provider trait remains for test/mock/local-model backends, not as a proxy landing path.
- **Embedded key in distributed binaries / shared default key**: **prohibited** (MCA §2.4). No secrets in code, logs, defaults, or shipped artifacts; key comes from explicit user config / OS keychain only. Binaries must not contain a fallback key.

## Consequences

### Positive

- Smallest P3 scope: no billing/auth service required to get L3 working.
- Privacy-local default: user key, user quota, user audit trail; no third-party hop.
- No Jev-proxy path in the backend at all: no proxy auth/billing/quota/abuse-limit design, build, or audit surface — backend scope is smaller than the draft assumed.

### Negative

- Each user must obtain and configure a Jev key (onboarding friction; `algo doctor` must
  diagnose missing/invalid keys as `ask`, never `allow`).
- Key storage and rotation burden falls on the client (keychain integration per OS).
- Team-wide policy/quota management goes through org-supplied keys (Algorithco subscription), not through a proxy we operate.

## Open questions (honest, unresolved)

- Key storage: OS keychain integration per platform `[VERIFY]` — 2026-09-20 — official
  keychain/credential-manager docs (links on verification; no API assumed).
- Jev API/SDK shape and ToS for key use from a local guard `[VERIFY]` — 2026-09-20 —
  official TypeSafe AI Jev docs (link on verification; P0-JEV-1 dossier decides).
- Proxy design: none — proxy mode will not be built (decided 2026-09-23); no proposal here or later.
- Cost/latency budget impact of BYOK (P0-JEV-2 measures BYOK path only; there is no proxy path to compare).

## Verification

- P0-JEV-1 dossier links official Jev API/SDK + MCA Sep 19, 2026 clauses (§§2.3(b),(g), 2.4, 4.1/4.3, 5, 12.3, 13.2, 16.4) + DPA Apr 24, 2026 with dates; P0-JEV-2 records
  BYOK false-allow + p50/p99 latency artifacts (no embedded key).
- Provider trait compiles with BYOK backend; proxy backend is a stub or missing — no
  dead proxy code paths that could fail open; **binary scan shows no embedded `TYPESAFE_API_KEY` / `ALGO_JEV_API_KEY` literal**. All timeout/parse errors prove `ask`.
- `algo log --show-egress` demonstrates redacted egress on the BYOK path; `local-only` default shows no egress.
- Evidence dossiers (not duplicated here): ToS verdicts in `docs/verify/jev-tos.md`
  (Q1–Q7 + sign-off), API/SDK/regions in `docs/verify/jev-api.md`, live blocker
  status in `docs/DEFERRED.md` §4.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/backend` + `@algorithcoguard/security` + `@algorithcoguard/legal` — required
  before any P3 L3 network scope or **proxy mode** work. Without it, no L3 provider implementation
  beyond the Phase 0 redacted probe client, and no proxy design/build. BYOK default itself needs legal sign-off per MCA §2.4.