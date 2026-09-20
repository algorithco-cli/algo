# DEFERRED — skipped P0 gate sign-offs and documentation debt (waiver ADR-0009)

> **Waiver:** `docs/adr/0009-p0-gate-waiver.md` (owner-instructed 2026-09-21, countersignature blank).
> P0 exit gate (`docs/exit-gate-P0.md:5`) is **deferred, not cancelled**. Every item below was **open as of `642b4e6` / `f198e97`** and is **skipped for the private MVP**.
> No sign-off field is filled by this file; all `__________` stay blank.
> **Rule:** an item must be done **BEFORE** the milestone in its row. No milestone ships without its block.

## How to read

- **Milestone:** the earliest public/real-data milestone that **requires** the item.
  - `public release` — anything visible outside the private monorepo (docs, binaries, API, demo).
  - `real user data` — any Jev call with a real user command (even `redacted` shadow).
  - `Jev enabled` — Jev does any `allow`/`deny` that is not shadow (`shadow:true` only until this).
  - `proxy mode` — traffic through `guard-backend` instead of BYOK.
- **Status:** all `pending` as of waiver. Update this file at each milestone (per owner).

## 1. Thresholds, ratifications, verdicts, gate sign-off

| # | File:line (as of waiver) | What is skipped | Milestone it must be done BEFORE |
|---|---|---|---|
| 1.1 | `docs/exit-gate-P0.md:40-44` — `strict`/`balanced`/`fast` thresholds (3× `ratified by __________ on __________`) | Per-profile false_allow ceilings ≤0.3%/≤1%/≤2% are **PROPOSED, not ratified** | `public release` and `Jev enabled` (gate cannot be enforced without a ratified ceiling) |
| 1.2 | `docs/exit-gate-P0.md:81-87` — §1.5 utility **G1** `false_ask ≤0.30 PROPOSED` + **G2** `Δ at fixed ask` vs realistic `rules_only` v0.2.0 | Utility gating that prevents always-ask (1.0) from passing on safety alone | `public release` and `Jev enabled` |
| 1.3 | `docs/exit-gate-P0.md:109-116` — §2 verdicts (4 rows: auto-approve, verifier, loop, scanner → `TBD`) + owners/issues | No capability is `go` / `narrow-scope` / `redesign` yet | `public release` and `Jev enabled` (auto-approve) |
| 1.4 | `docs/exit-gate-P0.md:186-199` — gate sign-off table: Gate owner / Security reviewer / Eval owner (3× `__________`) + `135-148` §3a/§3b boxes unchecked | P0 exit gate is **unchecked, unsigned** | `public release` (gate must be checked) and **private MVP may proceed only under this waiver** |
| 1.5 | `eval/datasets/v0.1/DATASET.md:16-26` + `eval/datasets/v0.1/REVIEW-PACKET.md:42-46` — human-vs-agent κ (overall + (a) non-ambiguous + (b) ambiguous) + `HUMAN-REVIEW.md` not yet filed | Second human grader for 101 blinded records, 3-way κ ≥0.70, per-record adjudication | `public release` (dataset is 100% synthetic agent labels until then) |
| 1.6 | `eval/questions/EVAL-6-STARTED.md:134-160` — 2–3 week time-box + decision rule (if no added value → verifier/scanner) | Question tuning (13 cells + boolean set + threshold sweep) not yet run on dev; held-out not yet scored; `HUMAN-REVIEW.md` gates it | `public release` and `Jev enabled` (tuned `questions-v0.1` must beat `rules_only` at comparable ask before Jev does `allow`) |

## 2. Dataset expansion (statistics, not yet demonstrated)

| # | File:line | What is skipped | Milestone |
|---|---|---|---|
| 2.1 | `docs/exit-gate-P0.md:30-37` + `eval/questions/EVAL-6-STARTED.md:113-120` — Option **A+C PROPOSED**: 300 dangerous for balanced/fast (~2–3 days) + strict deterministic (no Jev claim at strict); **B 1000 not started** | n=72/72 dangerous → 95% upper bound 4.1% (Clopper-Pearson), not ≤0.3%/1% (need ~997/299 at 0 obs) | `public release` (to claim balanced/fast) and `Jev enabled` |
| 2.2 | `eval/datasets/v0.1/DATASET.md:27-37` — real-session held-out (≥24 real consented redacted commands in held-out 71) | 0 real records today (240 synthetic / 0 real) | `real user data` (held-out must include real-session slice before any claim about real usage) |
| 2.3 | `eval/datasets/v0.1/DATASET.md:71-73` + `eval/questions/EVAL-6-STARTED.md:121-126` — generator diversity (≥2 generators + hand adversarial set, metadata `generator`) | Current dangerous are single-generator (agent) + no hand adversarial set logged | `public release` |

## 3. Redact, privacy, consent — first Phase 1 task (gates real data)

| # | File:line | What is skipped | Milestone |
|---|---|---|---|
| 3.1 | `docs/redact-consent-readiness.md:5-28` + `docs/redact-crate-design.md:1-98` — `core/crates/redact` (`aho-corasick` + regex, `<500µs/10KB`, proptest/fuzz, `proves_ask_on_no_redact`, `--show-egress` one path, `local-only` default) | **Written 2026-09-21** — `core/crates/redact/src/lib.rs` (9 tests: 7 unit + 2 proptest, `cargo clippy -D warnings` clean, **149µs/10KB** release <500µs) — **queued for human review**, still needs `--show-egress` wiring in `agent` CLI and `local-only` default before real data | `real user data` and `Jev enabled` — **no real user command may be sent to Jev (even shadow) until this crate + `--show-egress` + `local-only` default + opt-in consent are shipped + human-reviewed** (`redact-consent-readiness.md:38-41`) |
| 3.2 | `docs/privacy-dataflow.md:5-38` — consent drafts (US infra, unspecified retention "as long as reasonably necessary", non-US → US transfer, inspect step, `consent_id`) + `algo init` privacy prompt | Spec exists, not shipped | `real user data` |
| 3.3 | `docs/redact-consent-readiness.md:89-93` + `docs/redact-crate-design.md:10` + `docs/privacy-dataflow.md` sign-offs (4 lines) | Human review of redact+egress+consent design — blank | `real user data` |
| 3.4 | `docs/redact-crate-design.md:10` — `core/crates/redact` can start after human review of this plan | Design is draft, not yet reviewed | `real user data` |

## 4. TypeSafe confirmations + verification checks

| # | File:line | What is skipped | Milestone |
|---|---|---|---|
| 4.1 | `docs/verify/jev-tos.md:210-224` [VERIFY-OPEN-1] — written confirmation that **no AUP exists** (legal index only DPA/MCA/Privacy Policy, `typesafe.ai/legal/aup` 404, owner-confirmed 2026-09-20) | Dangling MCA §2.3(l) ref — written confirmation (email/ticket ID + date) not yet filed | `public release` (not a blocker per owner, but file the confirmation before public docs cite §2.3(l)) |
| 4.2 | `docs/verify/jev-tos.md:225-227` [VERIFY-OPEN-2] + `docs/verify/blocked-on-typesafe.md:R2` — DPA fixed retention/deletion SLA, subprocessors currency beyond 2026-09-20 snapshot, Telemetry opt-out/ZDR need (ZDR enterprise-only via `privacy@`) | Retention is "as long as reasonably necessary" (Privacy Policy) / "as long as necessary" (DPA Schedule I §8) — no fixed SLA; subprocessors snapshot 2026-09-20 (AWS / Modal/Nebius/CoreWeave / Slack/Google Workspace — all USA) | `real user data` (cannot promise a fixed retention to users) |
| 4.3 | `docs/verify/jev-tos.md:228-229` [VERIFY-OPEN-3] + `docs/verify/blocked-on-typesafe.md:R3` — distillation exception ticket (§2.3(b) Q7 wording) | L2 distillation is **PROHIBITED** on standard terms (`jev-tos.md:36-47`); ticket not yet filed | `Jev enabled` for L2 (Phase 3) — human-only redesign (`plans/phase-3-l2-model.md:1`) is controlling until written exception + superseding ADR |
| 4.4 | `docs/verify/jev-api.md:88-96` + `docs/verify/blocked-on-typesafe.md:R4` — serving regions / data residency beyond US hosting (all USA per owner, `vantage-*` labels in reports are not vendor regions) | Current `vantage-eu-central`/`us-east` are vantage labels, not vendor regions | `real user data` (if residency promise is needed) |
| 4.5 | `docs/verify/jev-api.md:83-86` — Telemetry opt-out / status / SOC2 / on-prem (D-02) | Not yet confirmed | `public release` (if docs claim opt-out or SOC2) |
| 4.6 | `docs/exit-gate-P0.md:170-180` — hook-matrix [VERIFY-OPEN] per cell (hook-payload `schema_version`, `adapter_version` min 2.1.191/195/274, Codex `notify` payload, OpenCode `permission.ask` contract) | 5 rows are doc-only with dated quotes, but 4 [VERIFY-OPEN] remain (owner + recheck 2026-10-20) | `public release` (if adapter claims blocking beyond the verified rows) |

## 5. Security, license, supply-chain, branch protection

| # | File:line | What is skipped | Milestone |
|---|---|---|---|
| 5.1 | `docs/threat-model-v0.md:58-59` — Human sign-off: `@algorithcoguard/security` + Confirm [VERIFY] links (2 boxes unchecked) | Threat model is draft, not human-reviewed | `public release` |
| 5.2 | `docs/adr/0001-license.md:5` — legal sign-off (permissive core/agent, D-07/D-B) — **Draft** | No LICENSE file; publish decision pending | `public release` (cannot publish as permissive before legal) |
| 5.3 | Branch protection — `gh api repos/algorithcoguard/algorithco-guard/branches/main/protection` 404 (public repo, `gh` check 2026-09-20) — CODEOWNERS review is **not enforced by GitHub** (`docs/github-org-plan.md:22-27`, `AGENTS.md:9`) | Relies on human discipline for deny list / thresholds / redaction / sig-verify / install paths / auth | `public release` (must be enabled — requires org Pro or public repo with protection — before any `core/`/`agent/` product code that is `public` and security-critical) |
| 5.4 | CI — `secrets` + `links` failed on `4d585cb` (`gh run list` — `secrets` missing `GITLEAKS_LICENSE` for org, `links` glob `ENOENT`), `eval` `pip install` flat-layout failed on `d6c606b` (`setuptools` discovery) | Fixed locally in `f198e97` (gitleaks OSS docker, globstar loop, `tool.setuptools.packages.find`) — **not yet verified as green in CI** on `origin/main` (`gh run list` last for `f198e97` not yet fetched) | `public release` (CI must be green on `main` before claiming gates) |
| 5.5 | Cargo gates for `core`/`agent` — `cargo test`/`clippy`/`deny`/`audit`/`fuzz`/`proptest`/`mutants`/`latency` — not run on product code (product code is stub, `core/.gitkeep` only, `cargo` never invoked in CI for `core/`) | `not verified` until `core/crates/redact` etc. land | `public release` |

## 6. Docs debt skipped under the waiver

| # | Item | Milestone |
|---|---|---|
| 6.1 | `docs/redact-crate-design.md` is **Draft** (not yet human-reviewed) | `real user data` |
| 6.2 | `docs/adr/0004-byok-vs-proxy.md`, `0005-core-distribution.md`, `0006-payments-mor.md`, `0007-windows-scope.md`, `0008-jev-shadow-only.md` — all **Draft** (only `0002-naming` **Accepted**) | `public release` |
| 6.3 | `docs/public-exposure-review.md` — 8 items that should not be public before license/legal (threat model, deny-list regexes, 240 adversarial records, `HUMAN-REVIEW.md`, verbatim MCA excerpts) — **Draft** proposal to keep `guard-eval` private + redact threat model | `public release` |
| 6.4 | `docs/STATUS-2026-09-20.md` — read-only audit, not a gate pass (gate stays `DRAFT` per `exit-gate-P0.md:4`) | `public release` |
| 6.5 | `docs/split-plan-draft.md` — **Draft split plan** (monorepo → `guard-*` repos via `git filter-repo`) is **superseded** by this waiver's "ONE repository (monorepo) stays PRIVATE until release" (owner decision 2026-09-21) — do not execute | `public release` (split only if later decided) |
| 6.6 | `docs/adr/0010-naming-amendment.md` — `algo guard` dispatcher / `guard-*` repos / `algorithco` org — **Draft superseded** by waiver "keep ADR-0002 unchanged: CLI `algo`, crates `algo-*`, `~/.algo/`" | `public release` |

## 7. How to update this file

- At each milestone, a human checks the row that was just satisfied and **checks a box in `docs/exit-gate-P0.md:5` or the relevant `docs/adr/*`**, **not** in this file. This file is the **inventory of what was skipped**; it is **not** the gate itself.
- `docs/SECURITY-REVIEW-QUEUE.md` is the companion — every security-critical file written under this waiver is listed there for later human review (CODEOWNERS on `deny_list`/`redact`/`sig-verify`/`install`/`auth`).

## Sign-off (leave blank — human act)

- [ ] This deferred inventory reviewed: __________ Date: __________
- [ ] Countersignature (waiver companion): __________ Date: __________
