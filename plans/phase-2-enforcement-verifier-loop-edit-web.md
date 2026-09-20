# Phase 2 — Enforcement + Verifier + Loop + Edit/Write + Web

> Exit gate: prompt-reduction measured, false-allow within threshold, no fail-open in review+fuzz.

## P2-01 Proto deltas (first, blocks rest)

Add `tool.after` (edit/write: path, byte-range, new-hash, redacted-preview), `agent.stop` (session, task ref, diff ref, test-report ref), `Decision{confidence,source_level,latency_ms,reason}`, `VerifierVerdict{pass,fail_back_to_work,ask}`, `LoopAction{continue,retry_differently,stop,ask}`, `PolicyProfile{strict,balanced,fast}`. `buf lint/breaking` clean, tag. No hand-duplicated types.

## P2-02 Core thresholds

`decide_with_confidence(score, profile)` from eval artifact (not hardcoded); hard-deny short-circuit; error → ask with test. Unit + proptest boundaries; mutants on hard paths; eval gate.

## P2-03 Edit/write normalization

Abs path → project-relative, whitespace/hash/timestamp strip for L1 key; redact file previews; proptest never merges dangerous↔safe; fuzz 1h clean.

## P2-04 Enforcement wiring

L0→L1→L3→L4 with profile thresholds (L2 off until P3); `algo init` shadow default; `algo enforce on|off`, `algo status`, `algo why`; `algo pause` daemon-broken; SQLite WAL audit. Budgets L0/L1 <3/<10ms, L3 <250/<800ms criterion+hyperfine CI. `algo uninstall` restores backups.

## P2-05 Claude edit/write + stop adapter

Extend parse/render for edit/write + agent.stop; verifier fail → push back. [VERIFY] re-check hook docs, never guess.

## P2-06 Verifier (`agent/verifier`)

On `agent.stop`: (a) tests run? (b) diff matches task? (Jev Choice/Score batched, timeout → ask never allow). Quiet unless fail. Eval fake-done vs real-done; false-pass ≤ threshold.

## P2-07 Loop controller

Per-session failure fingerprints (same cmd+error ×N, tokens, wall-clock): retry_differently → stop → ask. SQLite persisted. Synthetic 5× identical fails → escalates; flaky fail-fail-pass → no trigger.

## P2-08 Web + docs (`web`)

Astro+Starlight: landing, install, privacy/data-flow (local-only/redacted/full), versioned per proto tag, install-script hosting, static only. Claims match measured numbers. Theme: single source `design-tokens.md` (Variant 1 DECIDED) via Starlight CSS var overrides (§4); decision examples use allow/ask/deny tokens.

## Testing

fuzz shell/fs/policy/redact, mutants deny, eval gate core+agent CI, headless Claude E2E, deny/audit/vet + clippy clean.
