# ADR-0004: Jev BYOK vs proxy (ship BYOK first)

## Status

draft — 2026-09-20 (D-A / D-01; owner @algorithcoguard/backend)

## Context

L3 judgments call Jev over the network (see `docs/threat-model-v0.md` B2). The choice shapes
P3 scope, privacy posture, and cost: who holds the API key, what leaves the machine, and who
pays. Source: `plans/90-crosscutting-gates-ux-decisions.md` §3 (D-01) and Phase 0 draft D-A.
Tracking: P0-JEV-1 / P0-JEV-2 (API/SDK dossier + measurement); no L3 product path may ship
without this ADR merged.

## Decision

Ship BYOK first; keep proxy optional later. The provider layer supports both modes behind a
trait, but Phase 0–P3 builds, docs, and UX assume the user supplies their own Jev key.
Redact-before-network applies in both modes; `local-only` sends nothing except via the
explicit Jev path; `algo log --show-egress` shows what left the machine.

## Alternatives

- **Proxy-first (backend holds a shared key, clients call backend)**: rejected for now. It
  pulls auth, billing, rate-limiting, and abuse handling into P3 scope and concentrates
  disclosure risk. Revisit if enterprise demand requires centralized billing.
- **BYOK-only forever (no proxy trait)**: rejected. It forecloses team/enterprise flows where
  a proxy with org policy is valuable. Keep the trait so a proxy can land without rework.
- **Implicit key (env-var sniffing / shared default key)**: rejected. No secrets in code,
  logs, or defaults; key comes from explicit user config / OS keychain only.

## Consequences

### Positive

- Smallest P3 scope: no billing/auth service required to get L3 working.
- Privacy-local default: user key, user quota, user audit trail; no third-party hop.
- Trait preserves optionality: proxy can be added as a second provider backend.

### Negative

- Each user must obtain and configure a Jev key (onboarding friction; `algo doctor` must
  diagnose missing/invalid keys as `ask`, never `allow`).
- Key storage and rotation burden falls on the client (keychain integration per OS).
- Team-wide policy/quota management is deferred with the proxy.

## Open questions (honest, unresolved)

- Key storage: OS keychain integration per platform `[VERIFY]` — 2026-09-20 — official
  keychain/credential-manager docs (links on verification; no API assumed).
- Jev API/SDK shape and ToS for key use from a local guard `[VERIFY]` — 2026-09-20 —
  official TypeSafe AI Jev docs (link on verification; P0-JEV-1 dossier decides).
- Proxy design (if later): auth, per-org quota, abuse limits, audit — no proposal here.
- Cost/latency budget impact of BYOK vs proxy (P0-JEV-2 measures BYOK path only for now).

## Verification

- P0-JEV-1 dossier links official Jev API/SDK + ToS notes with dates; P0-JEV-2 records
  BYOK false-allow + p50/p99 latency artifacts.
- Provider trait compiles with BYOK backend; proxy backend is a stub or missing — no
  dead proxy code paths that could fail open. All timeout/parse errors prove `ask`.
- `algo log --show-egress` demonstrates redacted egress on the BYOK path.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/backend` + `@algorithcoguard/security` — required
  before any P3 L3 network scope or proxy work. Without it, no L3 provider implementation
  beyond the Phase 0 redacted probe client, and no proxy design/build.