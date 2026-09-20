# Design Tokens — Verification (`docs/design-tokens-verify.md`)

> Canonical source: `plans/design-tokens.md` (Variant 1 DECIDED, verified 2026-09-20).
> Built artifact: `design-tokens.css` (root). Consumers: `dashboard`, `web`, `docs`.

## 1. Grep check (no hard-coded hex outside vars)

Run from workspace root (and per-repo once they exist):

```powershell
# PowerShell — find stray 6-digit hex in source (should return ONLY var definitions + this doc)
Select-String -Pattern '#[0-9A-Fa-f]{6}' -Path design-tokens.css, docs/design-tokens-verify.md
# Per-repo (Phase 2+ dashboard / web) — expect only the 9 light + 9 dark values via vars, no strays:
# Select-String -Recurse -Pattern '#[0-9A-Fa-f]{6}' dashboard/src web/src
```

```bash
# bash equivalent
grep -rniE '#[0-9a-f]{6}' design-tokens.css docs/design-tokens-verify.md
# grep -rniE '#[0-9a-f]{6}' dashboard/src web/src  # Phase 2+
```

Pass criteria: every hit is either a `--ag-*` definition in `design-tokens.css`,
a Starlight/Tailwind mapping that references `var(--ag-*)`, or this doc quoting values
for audit. Any literal hex in component styles = fail (use vars).

Expected values (audit list — do NOT copy into components):

- Light: `#6D4AFF` `#FAFAFB` `#FFFFFF` `#E6E5EE` `#17161F` `#6B6A7B` `#1E9E63` `#D99A00` `#E5484D`
- Dark: `#8E77FF` `#0E0D15` `#16151F` `#26243A` `#F4F3FF` `#9C9AB0` `#3DD68C` `#F5B82E` `#FF6B6F`

## 2. Screenshot checklist (per UI PR)

- [ ] Dashboard light screenshot vs hex picker matches all 9 tokens
- [ ] Dashboard dark screenshot vs hex picker matches all 9 tokens
- [ ] `algo why` / history badges: `allow` → `--ag-allow`, `ask` → `--ag-ask`, `deny` → `--ag-deny` in both themes
- [ ] Brand (`--ag-brand`) never used for decision semantics; decision tokens never used for branding/decoration
- [ ] Screenshots attached to PR (light + dark)

## 3. Contrast (AA note)

- Record `text/surface` and `muted/surface` contrast ratios for light + dark (target: AA for body text).
- Large-text-only exceptions need an ADR note; normal-text failures = fail.
- Tool suggestion: any WCAG contrast checker; paste ratios + tool name/date into the PR.

| Pair | Light ratio | Dark ratio | AA pass? |
|---|---|---|---|
| `--ag-text` / `--ag-surface` | TBD | TBD | TBD |
| `--ag-text-muted` / `--ag-surface` | TBD | TBD | TBD |

> Status (Phase 0): ratios TBD — no UI exists yet. Fill during P2-08 (`web`) / P3 (`dashboard`).

## 4. Change process

Any color change = PR to `plans/design-tokens.md` + `design-tokens.css` + light/dark screenshots
+ downstream `dashboard`/`web` rebuild. No per-repo token forks.
