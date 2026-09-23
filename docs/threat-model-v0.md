# Threat model v0 (draft — Phase 0)

> Status: draft. Needs human sign-off from `@algorithcoguard/security` before Phase 1. No product code depends on this yet.

## Assets

| Asset | Location | Impact if compromised |
|-------|----------|-----------------------|
| Agent configs (hooks, settings) | user config dirs per agent | attacker disables or bypasses guard |
| Hook -> daemon channel | socket `~/.algo/algo.sock` (Unix) / named pipe (Windows `[VERIFY]` — 2026-09-20 — https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes) | spoofed decisions, privilege escalation |
| Audit log | SQLite `~/.algo/audit.db` (WAL) | tampered history, lost accountability |
| Keys (Jev API key, signing keys, login tokens) | `~/.algo/` + OS keychain `[VERIFY]` | exfil, impersonation, policy forgery |
| Daemon binary + policy bundle | install dir + `~/.algo/` | malicious allow, persistent backdoor |
| Cache (fingerprints, prior decisions) | `~/.algo/` | poisoning -> false-allow |

## Boundaries + STRIDE-lite

### B1: hook client -> daemon (local IPC)

- Spoofing: rogue process imitates daemon. Mitigation: socket ownership + permissions check; fail to `ask` on mismatch. Windows pipe ACL `[VERIFY]` — 2026-09-20 — https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights.
- Tampering: event mutated in transit. Mitigation: short timeout, schema validation; parse failure -> `ask`.
- Denial: daemon down. Mitigation: try start once, else `ask`. `algo pause` always works daemon-broken.
- Info disclosure: args contain secrets. Mitigation: no logging of raw payloads; redact before audit preview.

### B2: daemon -> Jev (network)

- Disclosure: source code or secrets leave machine. Mitigation: redact-before-network; `local-only` mode sends nothing except via explicit Jev path; `algo log --show-egress` inspector.
- Spoofing / Tampering: fake Jev endpoint. Mitigation: TLS + pinned host; signature check where applicable; timeout/error -> `ask`, never `allow`.
- Denial: Jev slow or down. Mitigation: L0-L2 keep working; L3 timeout -> `ask`. Budget L3 p50 < 250ms / p99 < 800ms.
- Residency (informational, R4 CONFIRMED 2026-09-23 — not a mitigation to build): cross-border transfer to the US is inherent to the current Jev integration (US-only hosting, EEA/UK under SCCs/UK Addendum); no region-pinning is available. Disclosed in consent (`docs/privacy-dataflow.md`).
- Jev API shape `[VERIFY]` — 2026-09-20 — official TypeSafe AI Jev docs (link on verification; no SDK assumed).

### B3: daemon -> backend (opt-in cloud, Phase 3)

- Spoofing: fake backend pushes policy. Mitigation: signed policy bundles, verify before apply; rollback rejected.
- Tampering / Rollback: old policy replayed. Mitigation: version + monotonic check; rollback -> `ask` + alert.
- Disclosure: audit upload leaks code. Mitigation: redacted by default, opt-in only, never code in telemetry.
- Auth `[VERIFY]` — 2026-09-20 — Zitadel OIDC device-flow docs (link on verification).

### B4: agent adapter <-> agent (hook format)

- Spoofing: rogue tool impersonates a trusted agent (forged agent id / hook envelope) to smuggle disallowed actions past the adapter. Mitigation: per-agent adapter allow-list; strict envelope validation behind the `parse` trait; unknown agent id or malformed envelope -> `ask`.
- Tampering: hook payload mutated between agent and adapter (reordered args, injected flags, truncated JSON, unknown extra fields). Mitigation: schema validation + canonicalization; unknown fields -> `ask`; parse failure or depth/size over-limit -> `ask`, never `allow`.
- Denial: agent emits malformed, oversized, or high-rate hook events that stall the adapter and block L0–L2. Mitigation: bounded parse (size/depth/time limits); malformed -> `ask` with fail-safe default; flood -> shed load + `ask`. `algo pause` always works adapter-stalled.
- Info disclosure: hook event carries secrets, file contents, or prompt text that the adapter logs or forwards unredacted. Mitigation: treat args as data, never as policy; redact before audit preview and before any network path; no raw-payload logging.
- Hook format per agent `[VERIFY]` — 2026-09-20 — Claude Code hooks docs, Codex CLI config docs, OpenCode plugin docs (links on verification). Build adapters behind `parse/render` traits; unknown fields -> `ask`.

## Abuse cases

| Abuse | Mitigation or [VERIFY] |
|-------|------------------------|
| Prompt-injected args (instruction hidden in tool args) | treat args as data, never as policy; hard-deny patterns on syntax tree; uncertain -> `ask` |
| `base64` + `eval` obfuscation | decode-aware shell analysis (tree-sitter-bash); encoded-payload rule -> `deny` or `ask` |
| `${IFS}` / variable-expansion evasion | expand-aware fingerprint; structure match, not raw string |
| `sh -c` nesting | recursive parse; depth limit; over-limit -> `ask` |
| `curl ... | sh` exfil / pipe execution | network-pipe rule -> `deny` by default in strict/balanced |
| Daemon spoofing | socket ownership check; failure -> `ask` |
| Cache poisoning | normalized fingerprint + TTL + signed policy version; mismatch -> re-evaluate |
| Policy rollback | monotonic version; rollback rejected + alert |

## Sign-off

- [ ] Human sign-off: `@algorithcoguard/security` — required before Phase 1 product code.
- [ ] Confirm `[VERIFY]` links + dates in P0-JEV spike.
