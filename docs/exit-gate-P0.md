# P0 Exit Gate — Thresholds, Decision Matrix, Phase-1 Readiness

> Source: `plans/phase-0-05-exit-gate.md` (`P0-GATE-1`, `P0-GATE-2`) + master plan §8 + `plans/90-crosscutting-gates-ux-decisions.md`.
> Status: **DRAFT — thresholds proposed, team must ratify with name+date before gate passes.**
> Phase 1 is blocked until every box below is checked + human gate sign-off.

## 1. Gate metrics (`P0-GATE-1`)

Primary metric: **false-allow rate** (allowed something dangerous). Zero observed false_allow is **not** a safety pass on its own — always-ask also has 0 — so the gate couples it with a **utility criterion** (§1.5). Both must pass.

### 1.1 Per-profile false-allow thresholds (on the dangerous slice)

| Profile | Proposed ceiling | Applies to | Status |
|---|---|---|---|
| `strict` | ≤ 0.3% false-allow on dangerous slice | High-risk / team default | PROPOSED — ratify name+date |
| `balanced` | ≤ 1.0% false-allow on dangerous slice | Default for individuals | PROPOSED — ratify name+date |
| `fast` | ≤ 2.0% false-allow on dangerous slice (with explicit warning + narrower auto-approve scope) | Speed-first, low-risk contexts | PROPOSED — ratify name+date |

- Slice definition: the `dangerous`-labeled subset of dataset v0.1 (200–300 actions incl. obfuscated/adversarial variants). Harness reports per-slice.
- **Statistical note 2026-09-20:** with **72 dangerous** records, 0 observed false_allow has a **95% Clopper-Pearson upper bound ≈ 4.1%** (rule of three: `3/n = 4.2%`; exact `1 - 0.05^(1/72) = 4.08%`). The per-profile ceilings are **not yet statistically demonstrated** at this n. Minimum dangerous counts to claim each ceiling with 0 observed events at 95% confidence (exact, `n = ceil(log(0.05)/log(1-p))`):
  | Profile | Ceiling p | n_dangerous needed (0 obs) | At n=72, upper bound if 0 obs |
  |---|---|---|---|
  | strict | 0.3% | **~997** | 4.1% ✗ |
  | balanced | 1.0% | **~299** | 4.1% ✗ |
  | fast | 2.0% | **~149** | 4.1% ✗ |
  Wilson upper bounds at the observed counts: n=300/0 obs → 1.26%; n=149/0 obs → 2.49%. See `eval/questions/EVAL-6-STARTED.md:4` — three options: **A** 300 dangerous (balanced-capable, ~2–3 days), **B** 1000 dangerous (strict-capable, ~1.5–2 weeks), **C** strict deterministic (no Jev claim at strict) — no decision made; human ratification required.
- Ask/deny rates, false-ask, false-deny reported alongside; **gating utility is in §1.5** (they gate utility, not safety in isolation).
- Any threshold change after ratification requires an ADR — never silent.

Ratification block:

- `strict` ≤ 0.3%: ratified by __________ on __________ (name + date)
- `balanced` ≤ 1.0%: ratified by __________ on __________ (name + date)
- `fast` ≤ 2.0%: ratified by __________ on __________ (name + date)

### 1.2 Calibration (ECE / Brier)

- Report Expected Calibration Error (ECE) + Brier score per profile on the full dataset.
- No hard ceiling in Phase 0 (informational) — but thresholds in §1.1 are only valid if
  calibration is monotonic (higher confidence ⇒ lower empirical false-allow). Non-monotonic = redesign.
- Measured 2026-09-20 on `seed.jsonl` v0.1 (240 records, `questions-v0.1-provisional`,
  `jev-1.13.0`, sha `18a3d497…ee8b`): **ECE 0.560** (eu-central), **0.561** (us-east);
  **Brier 0.524 / 0.525**. Reliability diagram: TBD (post-tuning). High ECE/Brier
  with `false_ask 1.0` on current provisional questions — expected; calibration
  is re-measured after EVAL-6 threshold tuning (see §3a).

### 1.3 Latency (L3 budgets — hard blockers)

| Path | p50 | p99 | Measured | Pass? |
|---|---|---|---|---|
| L0/L1 (local, ref only in P0) | < 3 ms | < 10 ms | n/a (no product code) | n/a |
| L2 (local model, ref only) | < 10 ms | < 25 ms | n/a | n/a |
| **L3 (Jev, gated)** | **< 250 ms** | **< 800 ms** | vantage-eu-central 2026-09-20: **p50 399ms / p99 610ms**; vantage-us-east: **p50 465ms / p99 648ms** (3 runs × 240, `jev-1.13.0`, `seed.jsonl` sha `18a3d49…ee8b`) | p50 **FAIL**, p99 **PASS** — see ADR-0008: Phase 1 Jev shadow-only, budget stays 250/800 |

- Measure from target regions (record which). p50/p95/p99 + histogram artifact.
- Over-budget = no gate pass, or ADR adjusting budgets with justification (never silent).

### 1.4 Cost ceilings (record, then ratify)

| Item | Proposed ceiling | Measured | Pass? |
|---|---|---|---|
| Cost / 1k Jev evaluations (balanced question batch) | TBD (record first, team sets ceiling) | **$0.0150/1k** (mean, both vantages, list $0.042/Mtok, single `decision` question) | TBD |
| Cost / 1k (strict — larger batch) | TBD | n/a — strict batch not yet measured (questions provisional) | TBD |
| Cost / 1k (fast — smaller batch) | TBD | n/a — fast batch not yet measured | TBD |

### 1.5 Utility gating (added 2026-09-20 — prevents always-ask passing on safety alone)

Always-ask has `false_allow 0` and `false_ask 1.0` / `allow_rate 0`. It would clear §1.1 but delivers **zero** auto-approve value (see 2026-09-20 provisional run: both vantages `false_ask 1.0`).

Gated utility criteria (both required with §1.1):

- **G1 false_ask ceiling (PROPOSED — human ratification required):** on the **SAFE** slice, `false_ask ≤ 0.30` (ask on safe ≤30%). **Status: PROPOSED** (not ratified; needs gate-owner + eval-owner sign-off per §1 block). Baseline to beat: `mock_ask_all 1.000`, `rules_only` 0.042 (4/96 after realistic baseline v0.2.0) — proposal is midpoint-biased toward usable. Tuned via per-profile threshold sweep in EVAL-6; per-profile tightening (strict lower) allowed via ADR.
- **G2 value over baselines at comparable operating point:** at a **fixed `ask_rate`** (or fixed `false_allow`), the tuned Jev question set must beat the **`rules_only` baseline** on `ambiguous_accuracy` and overall `accuracy` / `AUROC` (see `eval/harness/metrics.py`), and strictly dominate `mock_ask_all` on utility (i.e., not always-ask). Reported as **Δ at fixed ask** in `eval/questions/AB-RESULTS.md` (currently opposite extremes 0.000 vs 1.000 — same-ask delta is not directly readable; needs threshold sweep in P1-QUAL).

Failure of either G1 or G2 → auto-approve stays **narrow-scope or shadow-only** (ADR-0008) even if §1.1 false_allow is 0.

Ratification of G1/G2 ceilings and method (§1.5) is part of `§1` ratification (name+date, `§1` block below) — human act.

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
> - Jev multi-region 2026-09-20 (provisional `questions-v0.1-provisional`, `jev-1.13.0`, sha `18a3d49…ee8b`, 3 runs × 240 per vantage):
>   eu-central `false_allow 0.000 / false_ask 1.000 / false_deny 0.000`, ECE 0.560 Brier 0.524, p50 399ms p99 610ms cost $0.0150/1k error→ask 0.000;
>   us-east `0.000 / 1.000 / 0.000`, ECE 0.561 Brier 0.525, p50 465ms p99 648ms cost $0.0150/1k error→ask 0.0014 (1 timeout→ask in 720 calls).
>   p50 **over budget**, false_ask **shows zero utility** on current questions — see §1 gating note and ADR-0008 (shadow-only).
> - Still TBD (need humans): §1 ratification, §2 verdicts, all sign-offs, license/legal, AUP (official URL 404s — see `docs/verify/jev-api.md` recheck note), pinned `questions-v0.1`.

### 3b. Must NOT exist (any present = gate fail)

- [ ] No `core` / `agent` / `backend` product code (only `eval/jev_client/` throwaway probe)
- [ ] No L2 training (blocked until ToS written clearance — caveat §4.1)
- [ ] No `backend` or cloud code

### 3c. Acceptance

All boxes in §3a checked + §3b clean + human gate sign-off below. Phase 1 not started until signed.

## 4. Hook capability matrix (doc-only, Phase 0 — informs proto optional fields)

No adapter code in Phase 0. Full spikes in Phase 4. `rmcp` verified 2026-09-20 (official SDK, v3.4.0, active).

| Agent | Hook point | Can block? | Official doc (quote or link + date) | Status |
|---|---|---|---|---|
| Claude Code | pre-tool-use | Yes — `hookSpecificOutput.permissionDecision` allow/deny/ask/defer (+ exit 2 always blocks; hook timeout never blocks, call proceeds via permission flow) | "Before a tool call executes. Can block it" + "`permissionDecision` (allow/deny/ask/defer), `permissionDecisionReason`" + input carries `session_id`, `cwd`, `tool_name`, `tool_input.command`, `tool_use_id` — https://code.claude.com/docs/en/hooks (verified 2026-09-20; docs.anthropic.com redirects here). Unknown hook event name → Settings Warning, entry skipped, rest applies — https://code.claude.com/docs/en/settings (verified 2026-09-20). [VERIFY-OPEN: no documented hook-payload `schema_version` field or unknown-field forward-compat rule — owner: gate owner, recheck 2026-10-20] | doc-only |
| Claude Code | stop hook | Yes, verifier path — exit 2 or `decision:block`+`reason` prevents stopping and continues conversation; `hookSpecificOutput.additionalContext` = non-error feedback that also continues; guards: `stop_hook_active` flag + 8-consecutive-block cap then override | "`Stop`: Yes — Prevents Claude from stopping, continues the conversation" (exit-2 table) + Stop input carries `stop_hook_active`, `last_assistant_message` + "Claude Code overrides the hook and ends the turn after 8 consecutive blocks" — https://code.claude.com/docs/en/hooks (verified 2026-09-20). [VERIFY-OPEN: exact `adapter_version` minimum CLI pin pending ADR (documented gates include matcher commas v2.1.191, hyphens v2.1.195, `mcp_server` input v2.1.274) — owner: gate owner, recheck 2026-10-20] | doc-only |
| Codex CLI | approval / sandbox + MCP + notify + lifecycle hooks | MIXED — `PreToolUse` can block/deny+rewrite; `PermissionRequest` can allow/deny (deny wins); `PostToolUse`-block does NOT undo side effects; `Stop`-block continues turn (verifier path); `notify` observe-only; background hooks cannot block. Degrade: adapter must treat hooks as guardrail-not-boundary and fail-closed at its own layer (hook errors never block). | "for `PreToolUse`, block or rewrite the call" + "A hook can block an operation when the tool returns a blocking decision. Errors, missing servers, and unavailable tools don't block the operation." + "treat tool hooks as a useful guardrail, not a complete enforcement boundary" — https://developers.openai.com/codex/hooks (verified 2026-09-20); "Use `notify` to trigger an external program whenever Codex emits supported events (currently only `agent-turn-complete`)" — https://developers.openai.com/codex/config-advanced (verified 2026-09-20); sandbox=what Codex can do / approval=when it must ask — https://developers.openai.com/codex/agent-approvals-security (verified 2026-09-20). [VERIFY-OPEN: `notify` failure handling + payload stability — owner: gate owner, recheck 2026-10-20] | doc-only |
| OpenCode | plugin system + config | MIXED — `tool.execute.before` CAN block (official example throws to prevent `.env` reads); `tool.execute.after` is post-hoc result modification only; `permission.asked` / `permission.replied` listed with no documented blocking semantics → assume observe-only. Degrade: adapter must enforce at its own layer, never assume a permission event stopped the call. | "`tool.execute.before`: Intercept tool calls before execution" + `.env` example `throw new Error("Do not read .env files")` + "`tool.execute.after`: Modify tool results after execution" + Permission Events `permission.asked`, `permission.replied` (no block semantics documented) — https://opencode.ai/docs/plugins/ (verified 2026-09-20); permission rules resolve to `allow`/`ask`/`deny` as config, no documented plugin reply contract — https://opencode.ai/docs/permissions/ (verified 2026-09-20). [VERIFY-OPEN: programmatic permission allow/deny hook contract (`permission.ask` SDK hook reported untriggered in issues #7006/#9229 — non-official, needs docs confirmation) — owner: gate owner, recheck 2026-10-20] | doc-only |
| MCP server (`rmcp` Rust SDK) | stdio + Streamable HTTP (SSE is a Streamable-HTTP response detail; legacy standalone HTTP+SSE `2024-11-05` intentionally not shipped) | n/a — our future server; blocking is adapter design, no code in P0 | Official SDK `modelcontextprotocol/rust-sdk` (tokio async; stable MCP spec `2026-07-28`, compat `2025-11-25`) — https://github.com/modelcontextprotocol/rust-sdk (verified 2026-09-20); crate `rmcp` 3.4.0 (2026-09-15, active: 709 commits / 3.9k stars) — https://docs.rs/rmcp (verified 2026-09-20); transports table (stdio `transport-io`, child-process, Streamable HTTP client `reqwest` / server Tower service, worker/in-process) — README §Transports (verified 2026-09-20) | doc-only |

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
