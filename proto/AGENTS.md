# AGENTS.md — proto (`algorithco_guard.v0`)

> Contracts-only tree. No product code. Enforced per workspace `AGENTS.md`.
> Conflicts → stop + ask.

## 1. Contracts-first

- `.proto` is the contract. `buf breaking` must pass on every PR.
- Never duplicate proto-owned types: eval imports `dataset.proto`;
  `core`/`agent` generate at build from the tagged release; consumers pin exact tags.
- Never hand-edit generated code (`gen/`); fix the source, re-tag, regenerate.

## 2. `[VERIFY]` rule (no invented APIs)

- Hook formats, Jev API/SDK, library capabilities: verify against official docs
  (link + date). Unverifiable → `optional` field + `[VERIFY]` comment, behind a
  trait/interface downstream. Never guess required fields.
- `grep VERIFY` must not reach `main` unlinked on product paths.

## 3. Fail-safe + privacy

- Every new I/O / timeout / parse path proves `ask` downstream; error/unknown → ASK.
- Payloads REDACTED by default; FULL is explicit opt-in only.
- FREEFORM `AgentQuestion` is never auto-answered.

## 4. Workflow

- `buf lint` + `buf breaking --against '.git#tag=<latest>'` before every push.
- ADR-before-code for any `[DECISION]` item; no `D-*` implementation without merged ADR.
- Small PRs, conventional commits, `Assumptions:` section in PR description.

## Assumptions (fill per-PR)

- <!-- Ambiguous requirement + interpretation chosen. -->
- <!-- Anything [VERIFY-OPEN] and why still open. -->
