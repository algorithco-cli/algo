# P0-05 Exit Gate

## 1. Gate metrics (`P0-GATE-1`)

False-allow primary, ask/deny, calibration (ECE/Brier), latency vs L3 budgets, cost/1k. Thresholds per profile `strict/balanced/fast` (e.g. `balanced` false-allow ≤1% on dangerous slice, `strict` ≤0.3% — team ratifies with name+date). Latency/cost ceilings recorded.

Decision matrix per capability (auto-approve, verifier, loop, scanner → go / narrow-scope / redesign). Redesign path named for any failing capability. Latency/threshold changes require ADR, never silent.

## 2. Phase-1 readiness (`P0-GATE-2`)

Must exist: tagged proto, dataset v0.1, harness, Jev report, ADRs D-A..D-F (BYOK vs proxy, license, name, core distribution git-tags default, payments MoR defer+owner, Windows scope macOS+Linux), threat draft.
Must NOT exist: `core`/`agent`/`backend` product code, L2 training, backend.

Acceptance: all boxes checked + human gate sign-off. Phase 1 not started until signed.

## 3. Hook capability matrix (doc-only, Phase 0)

Claude pre-tool-use/stop, Codex approval/sandbox+MCP, OpenCode plugins — quote official docs (can-block / observe-only / unknown). Informs proto optional fields. No adapter code. Full spikes in Phase 4. `rmcp` maturity noted unverified.
