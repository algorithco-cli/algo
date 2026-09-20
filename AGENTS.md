# AGENTS.md — algorithco guard (workspace root)

> Working agreement for AI agents (+ humans). Enforced in every PR.
> Source: `plans/90-crosscutting-gates-ux-decisions.md` §10 + master plan §10.
> If this file conflicts with a `plans/*.md` file → **stop + ask**. Do not silently pick one.

## 0. How agents work here

1. **Read the plan + this file first.** Entry point: `plans/00-index-build-order.md`, then the
   phase file for your task (e.g. `plans/phase-0-02-proto-contracts-v0.md`), then the
   repo-local `AGENTS.md` (when inside `core/`, `agent/`, …).
2. **Conflict → stop + ask.** Plan vs `AGENTS.md` vs reviewer instruction: halt, state the
   conflict, ask the human. Never silently choose.
3. **`[VERIFY]` rule (no invented APIs).** Hook formats, Jev API/SDK, library capabilities:
   verify against official docs (link + date). If unverifiable → leave `[VERIFY]` /
   `[VERIFY-OPEN]` note and build behind a trait/interface. `grep VERIFY` must not reach
   `main` unlinked on product paths.
4. **Contracts-first.** `proto` is the contract. `buf breaking` must pass. Never duplicate or
   hand-edit proto-owned types; generate at build from a tagged `proto` release; consumers
   pin exact versions.
5. **ADR-before-code.** Any `[DECISION]` item (CEL vs DSL, BYOK vs proxy, ort vs candle,
   license, MoR, Windows scope, …) needs a merged ADR in `docs/adr/NNNN-title.md`
   (Status/Context/Options/Decision/Consequences/Verification) before implementation.
   Branch `adr/D0X-*`. CI blocks `D-*` implementation without a merged ADR.
6. **Fail-safe proves `ask`.** Every new I/O / timeout / parse path gets a
   `proves_ask_on_*` test (error → `ask`, never `allow`). Kill allow-on-error mutants.
7. **No secrets.** gitleaks pre-commit + CI. No secrets in code, logs, fixtures, or datasets.
   Datasets redacted; no user code without consent.
8. **Small PRs, conventional commits.** One repo per PR. Tests + docs updated. Linked
   tracking issue in `docs`. PR needs a benchmark or eval link + artifacts for any
   perf/accuracy claim. Install-path PRs include `algo init → algo doctor → algo uninstall → diff` E2E.
9. **Human sign-off (CODEOWNERS).** Deny list, thresholds, redaction, sig-verify,
   `algo init`/`algo uninstall`, auth — agent approval is never sufficient.
10. **Flag uncertainty.** PR description has an `Assumptions:` section. Ambiguous → ask,
    not silent choice.

## 1. Phase discipline (Phase 0 = now)

- **No product code.** `core/`, `agent/`, `backend/`, `dashboard/`, `web/` scaffold dirs are
  stubs. Only throwaway redacted probe client in `eval/jev_client/` (Phase 0).
- Task IDs: `P0-DOCS-*`, `P0-PROTO-*`, `P0-EVAL-*`, `P0-JEV-*`, `P0-GATE-*`.
- Build order (Waves 0–4): see `plans/phase-0-00-overview.md`. Critical path:
  labeling quality → harness → Jev measurement → gate.
- Do not start Phase 1 until `docs/exit-gate-P0.md` is fully checked + human gate sign-off.

## 2. Quality gates (CI merge-blockers)

| Gate | Where | Rule |
|---|---|---|
| fmt/lint/type | all | clippy `-D warnings`, Biome, ruff+mypy — fail on warn |
| unit+integration | all | cargo test, vitest/playwright, pytest |
| fuzz+property | core, agent | cargo-fuzz shell/fs/policy/redact + adapter parse; proptest fingerprint/redact/thresholds; nightly 1h + PR 5min smoke |
| mutation hard paths | core | cargo-mutants deny+threshold+sig-verify; survivor = fail |
| latency | core, agent | L0/L1 <3/<10ms, L2 <10/<25ms, L3 <250/<800ms; over-budget = no merge or ADR |
| eval false-allow | core, agent, eval | harness JSON vs per-profile threshold; regression = fail |
| buf lint/breaking | proto | every PR; tags generate Rust+TS |
| dep audit | all | cargo-deny/audit/vet, npm audit |
| agent E2E headless | agent | scripted incl. daemon-down/timeout/corrupt-model |
| reproducible+signed+SBOM | agent, backend | SLSA provenance, cosign/minisign verify, CycloneDX |
| human review CODEOWNERS | — | see §0.9 |

## 3. UX checklist (every user-facing PR, §7)

`algo init` ~30s · shadow-first + `would-have N` digest · `algo why` = action+reason+confidence+source+latency ·
profiles strict/balanced/fast visible · always-allow → local rule/signal (never overrides hard-deny) ·
status counts+savings · one-step `algo pause`/`algo uninstall` daemon-broken ·
privacy local-only/redacted(default)/full(opt-in) + `algo log --show-egress` ·
quiet unless attention · Variant 1 tokens only (`design-tokens.css`, no hard-coded hex).

## 4. Conventions

- Naming (DECIDED): product `algorithco guard`, CLI `algo`, proto `algorithco_guard.v0`,
  crates `algo-*`, home `~/.algo/`, socket `~/.algo/algo.sock`, DB `~/.algo/audit.db`.
- Colors: Variant 1 single source `plans/design-tokens.md` → `design-tokens.css`.
  Decision colors exclusive: allow/ask/deny only for decisions; brand never for semantics.
- Docs: `docs/adr/NNNN-title.md` per ADR; `agent-changelog/<phase>-*.md` per scaffold/change batch.
- PR template: `.github/PULL_REQUEST_TEMPLATE.md` — all boxes or explicit N/A + reason.

## 5. Assumptions (template — fill per-PR, delete this line in repo-local copies)

- <!-- List every ambiguous requirement + the interpretation you chose. Example: ... -->
- <!-- Anything marked [VERIFY-OPEN] and why it is still open. -->
