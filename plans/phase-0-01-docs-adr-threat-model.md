# P0-01 Docs, ADR Process, Threat Model

## 1. Docs repo skeleton (`P0-DOCS-1`)

- `README.md`, `AGENTS.md` (real runnable commands), `CODEOWNERS` (deny list, thresholds, redaction, sig-verify, install paths, auth), `CONTRIBUTING.md` (conventional commits, small PRs), `roadmap.md`.
- Acceptance: fresh-agent finds plan + ADR dir in <2 min; CODEOWNERS covers §6 list.

## 2. ADR process (`P0-DOCS-2`)

- `docs/adr/0000-template.md`: Context / Decision / Alternatives / Consequences / Verification.
- `adr/README.md`: ADR required for any `[DECISION]`, budget/threshold or fail-safe change. States: draft/accepted/superseded.
- `decision-log.md` with D-A..D-F + deferred (CEL/DSL → P1, ort/candle → P3, speculative eval → deferred).
- Acceptance: template merged, D-A..D-F drafts opened with owners/dates, markdown-link lint green.

## 3. Threat model draft v0 (`P0-DOCS-3`)

- Assets: agent configs, socket/pipe, SQLite, keys, daemon binary.
- Boundaries: hook→daemon→Jev→backend. STRIDE-lite per boundary.
- Abuse: prompt-injected args, obfuscated shell (`base64+eval`, `${IFS}`, `sh -c` nesting), exfil via curl pipe, daemon spoofing, cache poisoning, policy rollback.
- Acceptance: each boundary has threat + mitigation-or-`[VERIFY]`; human sign-off.

## 4. Security-model + privacy stub

- `security-model.md`: fail-safe, rules-outrank-models.
- `privacy-dataflow.md`: `local-only` / `redacted` (default) / `full` (opt-in), redact-before-network, inspect-what-would-send, telemetry opt-in never code.
- Acceptance: text matches future `algo init` behavior; no vendor claims without measurement link.

## 5. Tasks

- `P0-DOCS-4` License + naming ADRs (blocks proto tag): DECIDED — product `algorithco guard`, CLI `algo`, proto `algorithco_guard.v0`. Apply LICENSE headers. **Deps:** none. **Risk:** Low.
- `P0-DOCS-5` Tracking issues: one per workstream. **AC:** all P0 PRs link an issue.

## Non-goals

No product code, no L2 design beyond ToS note, no infra.
