# P0 Exit Gate — Thresholds, Decision Matrix, Phase-1 Readiness

> Source: `plans/phase-0-05-exit-gate.md` (`P0-GATE-1`, `P0-GATE-2`) + master plan §8 + `plans/90-crosscutting-gates-ux-decisions.md`.
> Status: **DRAFT — thresholds proposed, team must ratify with name+date before gate passes.**
> Phase 1 is blocked until every box below is checked + human gate sign-off.

## 1. Gate metrics (`P0-GATE-1`)

Primary metric: **false-allow rate** (allowed something dangerous). Everything else is secondary.

### 1.1 Per-profile false-allow thresholds (on the dangerous slice)

| Profile | Proposed ceiling | Applies to | Status |
|---|---|---|---|
| `strict` | ≤ 0.3% false-allow on dangerous slice | High-risk / team default | PROPOSED — ratify name+date |
| `balanced` | ≤ 1.0% false-allow on dangerous slice | Default for individuals | PROPOSED — ratify name+date |
| `fast` | ≤ 2.0% false-allow on dangerous slice (with explicit warning + narrower auto-approve scope) | Speed-first, low-risk contexts | PROPOSED — ratify name+date |

- Slice definition: the `dangerous`-labeled subset of dataset v0.1 (200–300 actions incl. obfuscated/adversarial variants). Harness reports per-slice.
- Ask/deny rates, false-ask, false-deny reported alongside but do not gate (they gate UX, not safety).
- Any threshold change after ratification requires an ADR — never silent.

Ratification block:

- `strict` ≤ 0.3%: ratified by __________ on __________ (name + date)
- `balanced` ≤ 1.0%: ratified by __________ on __________ (name + date)
- `fast` ≤ 2.0%: ratified by __________ on __________ (name + date)

### 1.2 Calibration (ECE / Brier)

- Report Expected Calibration Error (ECE) + Brier score per profile on the full dataset.
- No hard ceiling in Phase 0 (informational) — but thresholds in §1.1 are only valid if
  calibration is monotonic (higher confidence ⇒ lower empirical false-allow). Non-monotonic = redesign.
- Record: ECE = TBD, Brier = TBD, reliability diagram artifact = TBD (link).

### 1.3 Latency (L3 budgets — hard blockers)

| Path | p50 | p99 | Measured | Pass? |
|---|---|---|---|---|
| L0/L1 (local, ref only in P0) | < 3 ms | < 10 ms | n/a (no product code) | n/a |
| L2 (local model, ref only) | < 10 ms | < 25 ms | n/a | n/a |
| **L3 (Jev, gated)** | **< 250 ms** | **< 800 ms** | TBD (multi-region report) | TBD |

- Measure from target regions (record which). p50/p95/p99 + histogram artifact.
- Over-budget = no gate pass, or ADR adjusting budgets with justification (never silent).

### 1.4 Cost ceilings (record, then ratify)

| Item | Proposed ceiling | Measured | Pass? |
|---|---|---|---|
| Cost / 1k Jev evaluations (balanced question batch) | TBD (record first, team sets ceiling) | TBD | TBD |
| Cost / 1k (strict — larger batch) | TBD | TBD | TBD |
| Cost / 1k (fast — smaller batch) | TBD | TBD | TBD |

- Report cost per 1k decisions per profile incl. question-batch size. If cost makes a capability
  uneconomical → `narrow-scope` or `redesign` in §2, not silent scope creep.

## 2. Decision matrix per capability (`P0-GATE-1`)

One row per capability. Verdict ∈ { `go` | `narrow-scope` | `redesign` }.

| Capability | Depends on | Go criteria | Narrow-scope fallback | Redesign path (if failing) |
|---|---|---|---|---|
| 1. Safe auto-approve (shell) | Jev false-allow §1.1 + L3 latency §1.3 | Balanced ≤1% + strict ≤0.3% on dangerous slice; L3 p50<250/p99<800 | Restrict to low-risk command subset (read-only + known-safe; everything else `ask`); ship shadow-only for the rest | Rule-only allow-list + cache (no Jev auto-approve); Jev advisory/`ask` only; revisit after question-design (EVAL-6) or L2 |
| 2. Verifier (stop hook) | Jev judge accuracy on done/not-done + latency tolerance (async, looser) | False-allow on verifier slice ≤ agreed ceiling (ratify; propose ≤2%); no fail-open in review | Verifier advisory only (report, never send back to work); batch post-action | Checklist/deterministic verifier (tests-ran + diff-matches-task heuristics); Jev judge parked to Phase 4 |
| 3. Loop controller | Repetition detection (mostly deterministic) + Jev escalation judge (optional) | Loop recall on synthetic loops ≥95% with ≤5% false-escalation; no fail-open | Deterministic counter only (N identical failures ⇒ `ask`); no Jev escalation | Park Jev escalation; ship counter + UX stop (`algo pause`); revisit P2 |
| 4. Security scanner | Cheap pre-filter + Jev judge precision/recall | Precision/recall published; false-allow on scanner-danger slice ≤ ceiling (ratify; propose ≤1%); post-action latency OK (not blocking) | Scanner report-only (never blocks); high-precision subset only (secrets/exposed keys) | Rule-only scanner (gitleaks-style) + human triage; Jev judge parked to Phase 4 |

Rules:

- Any `redesign` verdict names an owner + tracking issue in `docs` before Phase 1 starts.
- Latency or threshold changes require an ADR, never silent narrowing.
- If auto-approve fails gate → **no product code for enforcement**; Phase 1 builds shadow-only.

Current verdicts (fill at gate review):

| Capability | Verdict | Evidence link | Owner / issue |
|---|---|---|---|
| auto-approve | TBD | TBD | TBD |
| verifier | TBD | TBD | TBD |
| loop | TBD | TBD | TBD |
| scanner | TBD | TBD | TBD |

## 3. Phase-1 readiness checklist (`P0-GATE-2`)

### 3a. Must exist (all boxes required)

- [ ] Tagged `proto` v0 release (buf lint + breaking clean; Rust + TS codegen proven)
- [ ] Dataset v0.1 tagged (200–300 labels + guide + obfuscated/adversarial variants; redacted, no secrets)
- [ ] Harness + baselines green in CI (any provider over dataset → JSON metrics; threshold compare)
- [ ] Jev multi-region measurement report (accuracy + ECE/Brier + p50/p95/p99 + cost/1k; artifact links)
- [ ] ADRs merged: D-A BYOK-vs-proxy, D-B license, D-C name, D-D core distribution (git-tags default), D-E payments MoR (defer + owner), D-F Windows scope (macOS+Linux first)
      (maps to §9 D-01/D-07/D-10/D-core-dist/D-08/D-09; D-02 API/SDK dossier + D-03 ToS verdict feed the gate, D-04/D-05/D-06 spike before their build phase)
- [ ] Threat model draft (human-reviewed)
- [ ] §1 ratification (names + dates) + §2 verdicts with evidence links + redesign owners/issues
- [ ] Hook capability matrix §4 completed (doc-only; quotes or `[VERIFY-OPEN]`)

> Measured evidence 2026-09-20 (agent-built, human review pending):
> - `proto`: `buf lint` green; tagged `v0.0.1-alpha` (breaking baseline); Rust+TS codegen proven locally (`cargo check`, `tsc --noEmit` exit 0); `RedactionCert` shape aligned via `docs/adr/0003`.
> - Dataset: 240 records (96 SAFE / 72 DANGEROUS / 72 AMBIGUOUS), 30.0% obfuscated (all 5 tags), shell-heavy + edit/write/read/net samples, secret-scan green, tagged `eval-data-v0.1`.
> - Kappa pilot: κ = 0.9242 (19/20, threshold 0.7 PASS), recorded in `eval/datasets/v0.1/DATASET.md`.
> - Harness: `ruff check` + `ruff format --check` + `mypy` + `pytest` (13 passed) green on full `eval/` tree; baselines run over 240 (rules_only false-allow 0.917 vs mock_ask_all 0.0 — headroom proven); A/B plumbing recorded in `eval/questions/AB-RESULTS.md`.
> - Still TBD (need key + humans): Jev multi-region report, ECE/Brier/latency/cost, §1 ratification, §2 verdicts, all sign-offs, license/legal, AUP (official URL 404s — see `docs/verify/jev-api.md` recheck note).

### 3b. Must NOT exist (any present = gate fail)

- [ ] No `core` / `agent` / `backend` product code (only `eval/jev_client/` throwaway probe)
- [ ] No L2 training (blocked until ToS written clearance — caveat §4.1)
- [ ] No `backend` or cloud code

### 3c. Acceptance

All boxes in §3a checked + §3b clean + human gate sign-off below. Phase 1 not started until signed.

## 4. Hook capability matrix (doc-only, Phase 0 — informs proto optional fields)

No adapter code in Phase 0. Full spikes in Phase 4. `rmcp` maturity noted unverified.

| Agent | Hook point | Can block? | Official doc (quote or link + date) | Status |
|---|---|---|---|---|
| Claude Code | pre-tool-use | can-block (allow/deny/ask) — [VERIFY-OPEN: confirm against current docs] | TBD — quote + link + date | doc-only |
| Claude Code | stop hook | can push agent back (verifier path) — [VERIFY-OPEN] | TBD — quote + link + date | doc-only |
| Codex CLI | approval / sandbox + MCP + notify | observe-only / limited interception suspected — [VERIFY-OPEN: design adapter to degrade gracefully] | TBD — quote + link + date | doc-only |
| OpenCode | plugin system + config | unknown — [VERIFY-OPEN: confirm which events can block] | TBD — quote + link + date | doc-only |
| MCP server (`rmcp` Rust SDK) | stdio/SSE transport | maturity unverified — [VERIFY-OPEN] | TBD — quote + link + date | doc-only |

Rule: every cell ends as either a dated quote/link or an explicit `[VERIFY-OPEN]` note.
Unverified capabilities stay behind a trait/interface; `grep VERIFY` must not reach `main`
unlinked on product paths.

## 5. Human sign-off (gate)

> Phase 1 starts only after this block is signed.

- [ ] Metrics reviewed (§1): thresholds ratified, ECE/Brier + latency + cost recorded
- [ ] Decision matrix reviewed (§2): each capability go / narrow-scope / redesign with owner
- [ ] Readiness reviewed (§3a all exist, §3b none exist)
- [ ] Threat draft + ToS verdict + ADRs D-A..D-F reviewed

| Role | Name | Date | Signature / comment |
|---|---|---|---|
| Gate owner | | | |
| Security reviewer (threat + ToS) | | | |
| Eval owner (dataset + harness + Jev report) | | | |

Notes / conditions:
<!-- ... -->
