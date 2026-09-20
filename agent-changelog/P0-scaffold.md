# P0 Scaffold — workspace + gates + org plan

Date: 2026-09-20 · Scope: `P0-DOCS-1` (skeleton) + `P0-GATE-1/2` (draft) · Phase 0, no product code.

## Added

- `README.md` — product/CLI/paths, multi-repo table, DAG, plan entry point
- `AGENTS.md` — working agreement (plan-first, stop+ask, VERIFY, contracts-first, ADR-before-code, proves_ask_on_*, gitleaks, small PRs, CODEOWNERS, Assumptions)
- `.github/PULL_REQUEST_TEMPLATE.md` — plan/ADR/buf/latency-eval/CODEOWNERS/uninstall-E2E/Assumptions checkboxes
- `docs/exit-gate-P0.md` — gate metrics (false-allow strict ≤0.3% / balanced ≤1% / fast ≤2% PROPOSED, ECE/Brier, L3 p50<250/p99<800, cost/1k), per-capability matrix, readiness checklist, hook matrix doc-only, sign-off block
- `docs/github-org-plan.md` — org `algorithcoguard` repo list, branch protection, proto-tag-first flow, `gh` create commands (documented, NOT executed), local stub dirs
- `design-tokens.css` — Variant 1 vars light+dark (from `plans/design-tokens.md`)
- `docs/design-tokens-verify.md` — grep check, screenshot checklist, contrast AA note
- Stubs: `core/` `agent/` `backend/` `dashboard/` `web/` each with `.gitkeep` + `README.md` (Phase 0 — no product code)

## Verified

- `gh auth status`: logged in as Hamroqulovv (second account token invalid — noted, no action)
- `gh repo list algorithcoguard`: only `algorithcoguard/.github` exists — no repos created (per rule)
- Listing: new files + 5 stub dirs present, each with `.gitkeep` + `README.md` only

## Assumptions

- Threshold numbers in `docs/exit-gate-P0.md` are proposals; team ratifies with name+date.
- Repo visibility in org plan is a proposal pending license ADR + legal sign-off.
- Hook matrix cells left `[VERIFY-OPEN]` until quoted from official docs with date.
- `fast` ≤2% ceiling assumes explicit warning + narrower scope; ratify or tighten.

## Next

- `P0-DOCS-2` ADR process + `docs/adr/` skeleton; `P0-DOCS-3` threat v0; `P0-DOCS-4` license+name ADRs
- `P0-PROTO-1` buf+CI; hook doc quotes to clear `[VERIFY-OPEN]` cells
- Fill ECE/Brier + latency + cost TBDs via `P0-EVAL-*` / `P0-JEV-*` measurement
