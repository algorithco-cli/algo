# ADR-0007: Windows scope (macOS + Linux first)

## Status

draft — 2026-09-20 (D-F / D-09; owner @algorithcoguard/agent)

## Context

Windows support (installer, daemon IPC, adapter paths) is P4 scope; Phase 0–P3 target
macOS + Linux (source: `plans/90-crosscutting-gates-ux-decisions.md` §3 D-09; Phase 0
draft D-F). IPC differs by OS (Unix socket `~/.algo/algo.sock` vs Windows named pipe),
and install/uninstall file-modification paths need per-OS hardening.

## Decision

Ship macOS + Linux first. Harden Windows in P4 unless demand pulls it forward via a
superseding ADR. Until then, Windows-specific paths stay behind traits, fail safe to
`ask`, and no Windows release or install-claim ships.

## Alternatives

- **All three OSes from day one**: rejected. Triples installer/IPC/CI matrix while the
  policy engine and eval gate are still unproven on the primary platforms.
- **Windows-first**: rejected. Primary agent dev loops measured in Phase 0 are macOS/Linux;
  no evidence justifies inverting the order.
- **WSL-only as Windows story**: rejected as the permanent answer. It may suffice for an
  early preview but is not a hardened install/uninstall + IPC story; P4 still required.

## Consequences

### Positive

- Narrow CI/test matrix (macOS + Linux) while latency, eval, and fail-safe gates mature.
- Windows risk contained: unknown/unimplemented paths resolve to `ask`, never `allow`.
- Pull-forward option preserved: a superseding ADR can promote Windows without rework
  if the traits hold.

### Negative

- Windows users unsupported until P4 (adoption cost; messaging must say so plainly).
- Deferred IPC/installer divergence risk: named-pipe ACLs and install paths still need
  design and testing later.
- Adapter coverage skewed to Unix shells until P4.

## Open questions (honest, unresolved)

- Windows named-pipe endpoint + ACL shape `[VERIFY]` — 2026-09-20 —
  https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipes and
  https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights
  (no hook/IPC facts assumed beyond these links).
- Install/uninstall scope per OS (`algo init`/`algo uninstall` backup/restore, PATH,
  signing) — P4 hardening checklist undefined here.
- CI runners and device matrix for P4 (versions, shells, privilege levels).
- Pull-forward criteria: what demand or customer signal justifies promoting Windows early.

## Verification

- CI matrix green on macOS + Linux; any Windows-only path is trait-gated with a
  `proves_ask_on_*` test (unimplemented → `ask`).
- No Windows release artifacts, install scripts, or install claims ship before P4.
- Threat-model B1/B4 IPC notes stay marked `[VERIFY]` where Windows facts are unconfirmed.

## Sign-off (leave blank — human act)

- [ ] Human sign-off: `@algorithcoguard/agent` + `@algorithcoguard/security` — required
  before any Windows release claim or any P4 Windows hardening scope change. Without it,
  no Windows installer/IPC implementation and no pull-forward out of P4 order.