# Security remediation plan

Status: active stop-ship plan  
Source: independent backend/core/agent review received 2026-09-29  
Release posture: keep the repository private and do not expose the backend or
claim enforcement until the Phase 0 exit criteria pass.

## Rules of execution

- One narrowly scoped pull request per row below; do not combine backend,
  agent, core, and frontend changes merely to reduce PR count.
- Any auth, policy signature, deny-list, redaction, install, or uninstall
  change requires CODEOWNERS human sign-off.
- Add a negative test for every authorization boundary and a
  `proves_ask_on_*` test for every new parse, timeout, or I/O failure.
- Decisions about the policy language, payment provider, trusted-proxy model,
  shell parser, key custody, or Windows support require a recorded owner decision before
  implementation. Record the owner decision in the PR description when the work is scheduled; do not
  reuse or amend the naming decision (final).
- Hook behavior must be checked against the official Claude Code hook
  documentation on the implementation date and proven with a recorded/live
  end-to-end test. A source-code unit test alone is not sufficient.
- Every PR description must contain `Assumptions:` and link its test, eval, or
  benchmark artifacts.

## Phase 0 — stop ship

| Order | Task | Scope | Required proof | State |
|---:|---|---|---|---|
| 1 | `P0-GATE-AUTH-STUB` | Delete all acceptance of `valid-token-*`; make tests mint signed sessions. | Stub, malformed, tampered, expired, and foreign-signed tokens return 401. | Implemented in working tree; backend tests green. Human auth sign-off pending. |
| 2 | `P0-GATE-STARTUP-SECRETS` | Require a 32-byte session secret and a valid policy signing seed before binding; cache each once. Test-only keys must compile only under `cfg(test)`. | Missing/short/malformed values prevent listener startup; production build contains no fallback seed. | Implemented in working tree; startup-process test still required. Human signature/auth sign-off pending. |
| 3 | `P0-GATE-TENANT-AUTHZ` | Persist organizations, memberships, and roles. Derive caller identity from the verified session; owner/admin-gate policy publication; member-gate reads, ingest, stats, and export. Remove all-org queries and the HTTP WAL drain. | Per-route owner/member/stranger/unknown-org matrix, including cross-tenant replay and audit injection. Restart durability test. | Partially implemented with an in-memory owner registry and route tests; persistence and roles remain. |
| 4 | `P0-GATE-POLICY-BINDING` | Treat signed `org_id`, full version, and expiry as authoritative. Reject non-JSON payloads, outer/inner mismatches, cross-org replay, rollback, and expired bundles. Add an explicit re-sign/rotation path. | Signature corpus, per-org rollback property tests, rotation overlap test, and mutation coverage. | Binding checks exist in the working tree; re-sign/rotation and mutation proof remain. |
| 5 | `P0-GATE-RATE-LIMIT` | Replace Authorization-header buckets with layered global, trusted-client-IP, account, and sensitive-operation limits. Bound Google pending-flow storage without evicting valid flows silently. | Unique-token rotation cannot raise the client budget; one attacker cannot exhaust all login/signup buckets; trusted-proxy spoof tests. | Not started. Trusted-proxy design requires owner decision. |
| 6 | `P0-GATE-BILLING-LOCK` | Disable manual activation and immediate upgrades until a verified payment webhook is authoritative. Enforce `period_end`; make create/check/insert atomic. Prevent trial farming by new org ids. | No request can create a paid entitlement without a verified webhook event; concurrent-create test; expiry test. | Not started. Payment/MoR decision requires owner decision and human auth/billing sign-off. |
| 7 | `P0-GATE-RELEASE-BLOCK` | Add a release gate that fails while any Phase 0 item is open; remove the production API console or build-gate it to development. | Production-mode smoke test exposes only intended routes and refuses incomplete configuration. | Not started. |

Phase 0 exit criteria: all seven rows are complete, relevant mutation tests have
no survivors, backend tests and clippy are green, and auth/signature changes
have recorded human approval.

## Phase 1 — make enforcement real

| Order | Task | Scope | Required proof | State |
|---:|---|---|---|---|
| 1 | `P0-JEV-HOOK-CONTRACT` | Route Claude `PreToolUse` input through the adapter; emit the documented `hookSpecificOutput` decision contract; install an absolute shipped hook path in supported settings files; cover Bash, Write, Edit, and MCP tools. | Recorded payload tests plus a live Claude Code deny/ask/allow E2E; daemon-down and missing-binary cases resolve to ask/block as documented. | Partial working-tree implementation; live E2E and install coverage remain. |
| 2 | `P0-GATE-RAW-DENY-FIRST` | Evaluate hard-deny rules on raw in-memory command data before redaction. Redact only storage and egress copies. | All credential-prefix command-chain regressions deny; audit and provider payloads contain no raw secret. | Not started. |
| 3 | `P0-GATE-SHELL-AST` | Replace whitespace/raw-substring checks with a real shell AST selected by owner decision. Unwrap wrappers and `-c`; model pipeline adjacency, redirection, quoting, and process substitution. | Bypass and false-positive corpora, parser fuzzing, property tests, mutation tests, and L0 latency artifact. | Not started; parser choice requires owner decision and deny-list human sign-off. |
| 4 | `P0-GATE-CACHE-SCOPE` | Cache only proven read-only allows; key by lossless command identity plus cwd, profile, policy version, tool/file context, and security-relevant environment. Never cache ask/deny as allow. | Pairwise non-collision properties for mode bits, paths, identities, env injection, and policy/profile changes. | Not started. |
| 5 | `P0-GATE-AGENT-SELF-PROTECT` | Remove relative/PATH daemon execution; protect guard state/settings from guarded tools; create home/db/socket with owner-only permissions and no chmod window; consistently honor `ALGO_HOME`. | Malicious-repo binary test, pause/settings/kill attempts, permission checks, and daemon-down E2E. | Relative spawn and Unix permissions are partially fixed in the working tree; self-protection remains. |
| 6 | `P0-JEV-PROVIDER` | Keep MockProvider out of enforce-mode production; require explicit provider configuration, HTTPS, bounded responses, and fail-safe mapping. Isolate API credentials from guarded processes. | Corrupt/slow/oversize/TLS/provider-error tests all ask; no key appears in env-visible child state, logs, or errors. | Partial provider feature exists; production wiring and credential isolation remain. |

Phase 1 exit criteria: the supported hook matrix enforces live, every failure
path proves ask/block, the deny bypass corpus is green, false-allow evaluation
does not regress, and L0/L1/L2/L3 budgets have artifacts.

## Phase 2 — durability and hardening

1. `P0-GATE-DURABILITY`: move orgs, memberships, users, subscriptions, audit,
   sessions/revocations, and rate limits to bounded durable stores; add restart,
   migration, retention, and multi-instance tests.
2. `P0-GATE-IDENTITY`: add email verification, password reset, logout/session
   revocation, uniform login responses/timing, Argon2 `spawn_blocking`, Google
   nonce ownership and one-shot redemption, JWKS refresh on unknown `kid`, and
   GitHub token audience/app binding.
3. `P0-GATE-HTTP-BOUNDS`: stream provider responses through a hard byte cap;
   bound audit export/stats work; remove unbounded clones and line reads.
4. `P0-GATE-WINDOWS`: implement and test named-pipe transport only after the
   Windows-scope owner decision is recorded.
5. `P0-GATE-SUPPLY-CHAIN`: commit binary workspace lockfiles, pin Actions by
   commit SHA, replace long-lived publish tokens with supported trusted
   publishing, and verify SBOM/provenance/signatures.
6. `P0-DOCS-SECURITY`: replace the template security policy with supported
   versions and a real private reporting channel; update stale repository and
   phase references.
7. `P0-PROTO-DASHBOARD`: make proto the single API contract, generate clients,
   align dashboard/backend routes, preserve merged headers, and replace browser
   `localStorage` bearer persistence with an owner-approved session design.

## Verification commands per change

Run only the subset touched by the PR, plus the security-specific test named in
that PR:

```powershell
# backend/core/agent
cargo test
cargo clippy -- -D warnings
cargo deny check
cargo audit

# proto changes
buf lint
buf breaking --against .git#branch=main

# documentation
npx --yes markdown-link-check docs/**/*.md
```

Before release, additionally run the PR fuzz smoke, nightly fuzz campaign,
core mutation suite, agent headless E2E (including daemon-down, timeout, and
corrupt-model cases), false-allow eval harness, latency benchmarks, gitleaks,
and signed reproducible-build/SBOM verification required by the root
[working agreement](../AGENTS.md).

