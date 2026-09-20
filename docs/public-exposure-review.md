# Public exposure review — what should not be public before license/legal (Draft)

> **Status:** draft — 2026-09-20 (owner: @algorithcoguard/legal + security)
> **Do not change visibility yourself.** This is a **Draft** for legal review.
> The repo `algorithcoguard/algorithco-guard` is **currently public** (verified `gh api repos/algorithcoguard/algorithco-guard --jq .visibility` → `public`).
> MCA §14.1 treats **agreement terms as confidential** (see below) — that plus threat/deny/adversarial details should not be public before clearance.

## 1. Rule that makes this urgent

- **MCA §14.1 (Confidentiality, https://typesafe.ai/legal/mca, Sep 19, 2026, fetched 2026-09-20):** "…TypeSafe's Confidential Information includes … the terms and conditions of this Agreement, and other non-public information with respect to the Services…" + §14.2 obligations. **Agreement terms are confidential.**
- **License ADR (`docs/adr/0001-license.md:5`) — Draft** (permissive core/agent, legal sign-off pending). No LICENSE file and no publish decision yet — the **visibility decision is still pending legal**.
- The monorepo is **public** today but contains the items below — legal should decide what moves to private (`guard-eval`, parts of `guard-docs`) or is redacted before the next public push/split.

## 2. Inventory — what is public today and should be reviewed

| # | Path (public on `main` as of `642b4e6`) | Why it should not be public before review | Sensitivity |
|---|---|---|---|
| 1 | `docs/threat-model-v0.md` (assets, 4 boundaries B1–B4, 8 abuse cases: injection, `base64+eval`, `${IFS}`, `sh -c` nesting, `curl|sh`, daemon spoofing, cache poisoning, rollback + mitigations) | Detailed attacker abuse inventory + mitigations — gives adversaries a checklist of what we block and how | **High** — move to `guard-docs` **private** or a private `security` repo until hardened, or redact to a public summary |
| 2 | `docs/redact-consent-readiness.md` + `docs/exit-gate-P0.md` hard-deny inventory (referenced `eval/baselines/rules_only.py` v0.2.0 patterns) | Deny-list details (`rm -rf /`, `mkfs`, `dd of=/dev/*`, `curl|sh`, `chmod 777`, `eval+base64`, `StrictHostKeyChecking` bypass, `nc -e`) + exact regexes in `eval/baselines/rules_only.py:17-45` (hard deny list) | **High** — `guard-core` deny rules should be public **eventually** (trust for a security tool), but **not before** CODEOWNERS + fuzz/mutants review; keep `rules_only.py` in `guard-eval` (private) until the product `core/crates/policy` deny list is human-reviewed (`AGENTS.md:9`); do not copy deny regexes to public docs beyond a summary |
| 3 | `eval/datasets/v0.1/seed.jsonl` (240 records, 72 DANGEROUS with 30% obfuscated, all 5 tags) + `eval/datasets/v0.1/review/review-sample-blinded.jsonl` (101) | **Adversarial data** — 72 dangerous payloads with obfuscated variants (how to encode, pipe, subshell) is a cookbook for bypass attempts | **High** — keep `guard-eval` **private** until at least `eval-data-v0.2` with human-reviewed + real-session held-out; public `guard-eval` should expose only a **small public sample** (e.g., 10 SAFE + 10 DANGEROUS sanitized) or stay private per `docs/github-org-plan.md:17` ("Datasets may stay private") |
| 4 | `eval/datasets/v0.1/HUMAN-REVIEW.md` (planned, 3-way κ + per-record adjudication, `review-sample-form.csv`) | Will contain per-record adjudicated labels + guide citations — reveals where our ambiguous boundary lies | **Medium-High** — keep in `guard-eval` private; public report is **aggregate κ + confusion matrix** (`HUMAN-REVIEW.md` abstract) |
| 5 | `docs/verify/jev-tos.md` **verbatim MCA excerpts** (§2.3(b), §2.3(g), §2.4, §4.1/§4.3, §5, §12.3/§13.2, §16.4 — ~15 quoted paragraphs) + `docs/verify/jev-api.md` + `docs/verify/blocked-on-typesafe.md:12-18` (clause inventory) | MCA §14.1 says agreement terms are **TypeSafe Confidential Information** — verbatim excerpts may exceed fair-use if expanded; also DPA Schedule I §8 verbatim | **Medium** — keep the **short fair-use excerpts** plus citations, but **remove long verbatim blocks** before public launch; replace with **summaries + links** (`https://typesafe.ai/legal/mca#2.3`) and a note "quotes are summaries; see MCA for full text." File a **TypeSafe permission** if long quotes are needed on public `web` |
| 6 | `docs/verify/jev-api.md`, `docs/verify/blocked-on-typesafe.md`, `docs/verify/distillation-wording-flags.md` (subprocessor list: AWS stores / Modal/Nebius/CoreWeave process-only / Slack/Google Workspace — all USA, from `trust.typesafe.ai/subprocessors` 2026-09-20) | Not highly sensitive, but **owner-confirmed subprocessors** should be cited, not copied verbatim beyond the list; trust page is public, but our **interpretation** of retention/ZDR is advisory | **Low** — can stay public, but mark as **owner-confirmed snapshot 2026-09-20** with link, not a stable fact |
| 7 | `docs/exit-gate-P0.md:30-37` (Clopper-Pearson upper bound 4.1% at n=72, sample-size table 149/299/997) + `eval/questions/EVAL-6-STARTED.md:113-120` (Option A/B/C effort estimates) | Not sensitive, but **synthetic-distribution caveats** could be misread as real-world guarantees if the repo is public and indexed | **Low** — keep public, but ensure the **synthetic-distribution caveat** travels with the numbers (`docs/exit-gate-P0.md:7-14` + `DATASET.md:14`) — already done |
| 8 | `eval/baselines/rules_only.py` v0.2.0 + `eval/harness/*` (provider trait, metrics) | Baseline logic is not sensitive, but **denial regexes + heuristics** are the same as #2 | **Medium** — if `guard-eval` stays private, keep baselines there; if `guard-core` is public, the product deny logic will eventually be public — sequence it **after** human review |

## 3. Proposal — what to move or remove (do not execute yet)

| Action | Repo visibility | What to do |
|---|---|---|
| Move **adversarial dataset** (`seed.jsonl` full 240 + review sample) to private | `guard-eval` **private** (already proposed in `docs/github-org-plan.md:17` and `split-plan-draft.md:1`) | Before split, keep monorepo public but **do not add more dangerous records** to public `seed.jsonl`; new Option A expansion (228 dangerous) should land **only** in `guard-eval` private. After split, `guard-eval` stays private; public gets a **sanitized 20-record sample** (`eval/datasets/v0.1/sample-public.jsonl`) with safe/obfuscated examples only. |
| Move **full threat model + hard-deny details** to private | `guard-docs` → split into `guard-docs` (public summary) + `guard-docs-private` or `guard-eval` private appendix | Publish a **public summary** (assets + 4 boundaries at high level, 8 abuses as categories without exact payloads) in `docs/threat-model-v0-public.md`; full `threat-model-v0.md` stays private until P1 exit. |
| **Redact verbatim MCA excerpts** to summaries + links | All `docs/verify/*` | Replace long quotes in `jev-tos.md:36-149` with 1–2 line summaries + `https://typesafe.ai/legal/mca#X.Y` anchors; keep short fair-use snippets (<25 words) where needed. Add footer: "This is a summary; see MCA/DPA for full text; MCA §14.1 — agreement terms are TypeSafe confidential information — reproduced here under fair-use for compliance review." |
| Keep **harness + baselines logic** public **or** private — choose one | If `guard-eval` private, harness stays private; if `guard-core` is public, its deny logic will be public post-review | Decision: **keep `guard-eval` private through Phase 1** (simplest; aligns with `AGENTS.md:7` "Datasets redacted; no user code without consent" + `DATASET.md:14` internal-only). Promote `core/crates/policy` deny list to public **only** after `cargo-mutants` + human review at P1 exit. |
| **Do not change visibility yourself** | Current `algorithcoguard/algorithco-guard` stays **public** until legal decides | Owner/legal decides per ADR-0001. If legal says "make private now," flip via `gh api repos/algorithcoguard/algorithco-guard -X PATCH -f visibility=private` **after** this review is signed. Do not delete history — use `MOVED.md` pointer if splitting. |

## 4. Immediate non-code steps (allowed now)

- Add a **private split** (`guard-eval` private) to the split plan (`docs/split-plan-draft.md:1` already does) and mark `seed.jsonl` as **internal-only** in `eval/datasets/v0.1/DATASET.md` (already `Treat as internal-only until then`).
- Open **tracking issues** in `guard-docs` private (or monorepo private label) for #1–#4 (threat-model redaction, dataset sample, MCA excerpt shortening, per-repo visibility decisions).

## Sign-off (leave blank — human act)

- [ ] Public/private split reviewed: __________ Date: __________
- [ ] MCA verbatim excerpt shortening approved: __________ Date: __________
- [ ] Visibility of `algorithcoguard/algorithco-guard` (stay public vs make private) decided: __________ Date: __________
