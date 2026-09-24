# Redact crate + consent/privacy readiness for Phase 1 Jev shadow mode

> **Status:** implementation landed 2026-09-24, human review pending.
> `core/crates/redact` + `--show-egress` + privacy/consent flow was the **first**
> Phase 1 implementation task — it gates any real user data leaving the machine.
> No real session command is sent to Jev (even `redacted` shadow) until this crate,
> the egress display, and the opt-in consent flow exist and a human has reviewed
> them (see §4 Gating). Implementation is done and green; review is not.
> All sign-off fields blank — human review required.

## 1. What Phase 1 shadow needs

Phase 1 shadow sends **redacted** `canonical.redacted_payload` + `tool_kind` + `agent_type`
to Jev (see `docs/privacy-dataflow.md:5-6`, `proto/algorithco_guard/v0/events.proto`,
`eval/jev_client/client.py` redaction hook). Real user commands enter this path
only in `redacted` or `full` (explicit opt-in) mode; `local-only` (the default
since 2026-09-24) never sends. `algo log --show-egress` shows the exact outbound
payload through the same redactor as the send path (one-path rule).

## 2. Redact crate — first Phase 1 task (requirements)

> **Ordering (PROPOSED, pending human review of this plan):** this crate (+ egress
> display + consent flow) is **Phase 1 task #1** — it gates every later Phase 1
> crate that touches real user data. No real session command is sent to Jev
> (even `redacted` or shadow) until the three items below exist, are wired,
> and are **human-reviewed** (CODEOWNERS on `core/crates/redact` +
> `agent/crates/cli-audit` privacy paths, per `AGENTS.md:60`).

| Item | Ready? | Evidence / gap | Requirement before any real-data egress |
|---|---|---|---|
| `core/` product code | **Shipped 2026-09-24, unreviewed** | `core/crates/redact/src/lib.rs` (AC pre-filter + 10 regexes + high-entropy pass, stable `<REDACTED:…>` placeholders, AWS docs-example exclusion per design:20). | Human review (CODEOWNERS) still required before any real-data egress. |
| Redaction patterns | **Partial (eval stub)** | `eval/jev_client/redaction.py:36-58` + `eval/tests/test_no_secrets.py:27-36` cover `AKIA`, `ghp_/gho_`, `xox*`, `-----BEGIN .*PRIVATE KEY-----`, `sk-live`, high-entropy — but eval-only, not the product crate, and no `aho-corasick` hot path yet. | Port to product crate behind trait, add `JWT-ish`, `AIza`, `password=|token=` patterns from plan; prove **<500µs/10KB**, **proptest** idempotence + no pattern survives, **fuzz** `cargo-fuzz` redact (no panic/OOM — see Requirement below). |
| Redact-before-egress wiring | **Gap** | `eval/jev_client/client.py:247-250` does redact-before-POST in the throwaway probe, but `core/crates/provider` Jev provider does not exist yet to own this invariant. | Provider trait (`core/crates/provider`) must enforce redact-before-network at the type level; audit inserts `redacted_command` + count, never raw (plan P1-04 AC). |
| `algo log --show-egress` | **Shipped 2026-09-24, unreviewed** | `algo log --show-egress` re-redacts stored commands through `Redactor::global()` (same fn as send path); `algo doctor --show-egress` previews masking on a sample secret payload and FAILs the check if it survives. | Human review still required before any real-data egress. |
| Tests | **Shipped 2026-09-24, unreviewed** | `cargo test -p algo-redact` 10 pass (incl. AWS docs-example exclusion, false-positive guards); proptest idempotence + no-survive; `fuzz/fuzz_targets/fuzz_redact.rs` (idempotence + no-survive asserts, type-checks); daemon `proves_ask_on_no_redact` (secret never reaches decision/audit) + `proves_ask_on_local_only_skips_judge`. | Human review still required before any real-data egress. |
| Default mode | **Shipped 2026-09-24, unreviewed** | Fresh `algo init` (incl. `--yes`) defaults to `local-only`; CLI/TUI/daemon read fallbacks are `local-only`; daemon stamps events from config and the pipeline **skips the L3 provider entirely** in local-only (`proves_ask_on_local_only_skips_judge`); unknown values normalize to local-only. | Human review still required before any real-data egress. |

### First-task exit criteria (all must be true before any real-data Jev call)

- [x] `core/crates/redact` + `--show-egress` + `local-only` default + opt-in consent flow implemented (2026-09-24, verified: cli 11 + daemon 49 tests green, init→doctor→uninstall E2E in temp HOME).
- [x] `cargo test`, proptest (idempotence, no pattern survives), `cargo-fuzz` redact (`fuzz_redact` target type-checks; nightly 1h + PR smoke per gate) — green.
- [x] Manual spot-check: `algo doctor --show-egress` masks the sample secret payload (secret-in → `<REDACTED:…>` out, FAILs otherwise).
- [ ] **Human review** of the three items above (CODEOWNERS on redaction/privacy) recorded with name+date — **no real user data goes to Jev until this review exists**.

## 3. Consent / privacy design readiness

| Item | Ready? | Evidence / gap | Needed before real-session shadow |
|---|---|---|---|
| Privacy modes | **Shipped 2026-09-24, unreviewed** | `algo init` prints the egress disclosure (US endpoint, SCC/UK-Addendum, unspecified retention, BYOK key) and records `consent.json` (`consent_id`, timestamp, scope `no-egress`/`jev-egress`); `full` requires typing FULL and rejects `--yes`. | Human review still required; consent wording approval is a human act. |
| Consent capture for real commands | **Partial 2026-09-24, unreviewed** | `~/.algo/consent.json` stores consent ID + timestamp + scope; revert = re-run `algo init` → local-only. Still missing: per-record consent linkage for real-session dataset rows + collector pipeline (still a gap). | Finish collector + record linkage, then human review, before any real-session collection. |
| Real-session collection pipeline | **Gap** | 0 real records (`DATASET.md:37`). No collector exists. | Build a consented collector (adapter → redact → redacted payload only) that the user can inspect and delete; no telemetry that contains source code unless explicitly opted-in. |
| AUP / DPA clearance | **Owner-confirmed 2026-09-20: no AUP exists** (legal index lists only DPA, MCA, Privacy Policy; `typesafe.ai/legal/aup` 404, MCA §2.3(l) dangling) — **not a blocker**, but written confirmation requested (`jev-tos.md:[VERIFY-OPEN-1]`); DPA fetched Apr 24, 2026 — retention is unspecified ("as long as reasonably necessary" / "as long as necessary") and distillation ticket TBD. | Full DPA review + ticket response before any real user traffic goes to Jev; AUP confirmation to file (not blocking). |
| Data residency / retention | **Partial — now US-confirmed** | Owner-confirmed 2026-09-20: hosted **US** (Privacy Policy "International Visitors"), **all subprocessors USA** (AWS stores / Modal/Nebius/CoreWeave process-only / Slack+Google Workspace support), retention is **unspecified** ("as long as reasonably necessary"), ZDR enterprise-only via `privacy@`. Current reports use vantage labels. | No fixed retention SLA to promise users — consent drafts already disclose "US infrastructure, unspecified retention, non-US → US transfer" (`docs/privacy-dataflow.md`). Confirm any residency options beyond US with `privacy@`/`sales@` if needed. |
| Human review of redacted payloads | **Gap** | No process to sample real redacted payloads for PII/secrets before inclusion. | Manual review queue for the ≥24 real held-out records before `eval-data-v0.2`. |

## 4. Phase 1 tasks blocked by missing TypeSafe data-handling answers (DPA / retention / AUP)

> To prioritize follow-up with TypeSafe. All sign-off fields blank — human act.
> No decision made; this is a blocking inventory, not a scope cut.

| # | Missing answer (owner) | Blocks Phase 1 tasks | What is blocked (details) | Follow-up ask (priority) |
|---|---|---|---|---|
| 1 | **AUP full read** — `typesafe.ai/legal/aup` is 404 (dangling MCA §2.3(l) ref, `docs/verify/jev-tos.md:88-93`, `jev-api.md:134-145`) — Owner: product/legal | EVAL-6 (real-session + dangerous eval traffic), P1-05 provider, P1-06 daemon L3, P1-08 shadow with real commands | Sending redacted **dangerous** commands to Jev for eval may violate AUP; shadow with real user `rm -rf` etc. is the same traffic | **P0** — locate current AUP URL, confirm redacted dangerous-command eval + shadow traffic is acceptable use (record URL+date in `jev-tos.md` [VERIFY-OPEN-1]) |
| 2 | **DPA — retention / subprocessors / deletion SLA** — `docs/verify/jev-api.md:80-86`, `jev-tos.md:101` — Owner: product/legal | P1-05 provider (any Jev egress), P1-06 daemon (L3 pool), P1-08 `audit.db` shadow + CLI `--show-egress` with real data, EVAL-6 real-session held-out | Standard MCA §4.1 gives TypeSafe a perpetual telemetry license (`jev-tos.md:35-45`); without retention/subprocessor/deletion terms we cannot promise users how long redacted states persist or where | **P0** — read DPA `typesafe.ai/legal/data-processing` (retention, subprocessors, deletion), record in `jev-tos.md` [VERIFY-OPEN-2]; ask about data residency/region exposure (`jev-api.md:90-96`) |
| 3 | **Telemetry opt-out / ZDR scope** — is Telemetry collection opt-out-able on standard plans? Is ZDR enterprise-only? — `jev-api.md:83-86` — Owner: product/legal | Same as #2, plus privacy-mode UX (`local-only` default, `redacted` vs `full`) | If standard plans cannot opt out of Telemetry (§4.3 "without restriction… to improve Services"), `redacted` mode still leaks telemetry; affects consent wording in `algo init` | **P0** — confirm opt-out + ZDR enterprise path (`privacy@` / `sales@`); document in `jev-api.md` [VERIFY-OPEN] + privacy-dataflow |
| 4 | **Distillation exception** — MCA §2.3(b) bans L2 training on Jev outputs (prohibited, `jev-tos.md:17`) — Owner: product | P3 L2 local classifier (`ort vs candle`, `core/crates/policy` L2) — not Phase 1 enforcement, but Phase 1 data-collection design (consent-only vs Jev-distilled) | L2 is **blocked** on standard terms; contingency is consent-only data (`jev-tos.md:77-83`). Until ticket answered, Phase 1 must not log Jev outputs for L2 | **P1** — file ticket "Does §2.3(b) prohibit training a small local safety classifier on Jev outputs for use inside our product? Is an enterprise amendment available?" Record ID in `jev-tos.md` [VERIFY-OPEN-3] |
| 5 | **Regions / residency / status / SLA** — serving regions, residency options, uptime SLA — `jev-api.md:90-96,122-126` — Owner: eval/product | EVAL-6 multi-region protocol (≥2 regions; current reports use vantage labels), P1-06 daemon region handling, P1-08 latency probe (`algo doctor`) | Current `vantage-eu-central` / `us-east` are vantage labels, not vendor regions; no SLA to promise users; retention/region interaction unknown | **P1** — ask `sales@`/`privacy@` for regions, residency, status page, SLA; file ADR if single-region only |

**What is NOT blocked (can proceed without these answers):**
- `core/crates/redact` itself (no egress, local-only) — first Phase 1 task (§2).
- `core/crates/types`, `fingerprint`, `shell-analysis`, `policy` (L0/L1 deterministic) — no network.
- `agent/crates/daemon` + `hook-client` + `adapter-claude` in **shadow-only with synthetic data** (no real user commands egress) — per ADR-0008.
- `eval` harness + baselines `rules_only`/`mock_ask_all` on synthetic `seed.jsonl` + dev/held-out synthetic split — no TypeSafe dependency.

## 5. Gating

Real user commands **must not** be sent to Jev in any Phase 1 build (even shadow) until:
all rows in §2 + §3 above are **shipped + human-reviewed** (CODEOWNERS on redaction/privacy paths,
per `AGENTS.md:60` / `docs/CODEOWNERS:11-12`), AUP/DPA/retention cleared (§4 gaps above),
and `docs/exit-gate-P0.md:5` signed. Until the first-task exit criteria are met,
`eval/jev_client/` remains the **only** Jev path (throwaway probe/client).
This report is advisory — it implements nothing.

## 6. Phase 1 implementation order (proposed)

1. **Redact crate + `--show-egress` + default `local-only` + opt-in consent** (this document) — human review gates step 2; see §2 First-task exit criteria.
2. `core/crates/types` + `fingerprint` + `shell-analysis` + `policy` + `provider` (mock + Jev behind trait, still `local-only` until consent).
3. `agent/crates/daemon` + `hook-client` + `adapter-claude` (shadow-only per ADR-0008) + `agent/crates/cli-audit`.
4. Quality/latency/eval gates (`AGENTS.md:52`) + shadow soak; then revisit EVAL-6 decision rule (2–3 week time-box, `EVAL-6-STARTED.md:7`).

All sign-off fields blank — human act. Product code exists (see `docs/exit-gate-P0.md` §3b enforcement verdict — gate FAIL until remediated); this document tracks only the redact/consent slice.

## Sign-off (leave blank — human act)

- [ ] Redact crate + egress + consent design reviewed: __________ Date: __________
- [ ] Privacy modes (`local-only` default, `redacted`/`full` opt-in) approved: __________ Date: __________
- [ ] No-real-data-to-Jev-before-review gate accepted: __________ Date: __________
- [ ] TypeSafe blocking list (§4) follow-up prioritized (P0/P1 owners assigned): __________ Date: __________
