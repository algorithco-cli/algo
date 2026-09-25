## Summary

<!-- What + why. Link the task ID, e.g. `P0-PROTO-2`. -->

Task ID:

## Checklist (all boxes or explicit N/A + reason)

- [ ] Scope matches the linked task (or deviation explained below)
- [ ] ADR linked if this touches a `[DECISION]` item (`docs/adr/NNNN-title.md` merged before code) — N/A if none
- [ ] `buf lint` + `buf breaking` pass (proto changes) — N/A if no proto change
- [ ] Latency / eval evidence linked (benchmark or harness JSON + artifacts) for any perf/accuracy claim — N/A if none
- [ ] Fail-safe: new I/O / timeout / parse paths have `proves_ask_on_*` tests (error → `ask`, never `allow`) — N/A if none
- [ ] No secrets (gitleaks clean; no secrets in code/logs/fixtures/datasets)
- [ ] Tests + docs updated; conventional commit; one repo per PR
- [ ] CODEOWNERS human review requested (required for: deny list, thresholds, redaction, sig-verify, `algo init`/`algo uninstall`, auth)
- [ ] Required status checks green per `docs/branch-protection.json` (CI Gate, Rust, Eval, Buf, Secrets, Web, Fuzz Smoke, Install E2E, Latency) — N/A needs reason
- [ ] Install-path E2E included (`algo init → algo doctor → algo uninstall → diff` proves reversible) — N/A if not install path
- [ ] UX checklist considered (§7: shadow-first, `algo why`, profiles, privacy modes, quiet-unless-attention, Variant 1 tokens) — N/A if not user-facing

## Verification

<!-- Commands run + results. Paste `buf`, `cargo test`, `vitest`/`playwright`, `pytest`, `gitleaks`, benchmark links. -->

```
```

## Assumptions

<!-- REQUIRED. List every ambiguous requirement + the interpretation you chose. Ask, do not silently choose. -->

- <!-- e.g. Assumed X means Y because <plan §>. -->
- <!-- [VERIFY-OPEN] items still open + why: ... -->

## Screenshots / artifacts

<!-- Light+dark screenshots for any color/UI change (must match `design-tokens.css`). Eval/bench artifacts otherwise. -->
