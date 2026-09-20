# Design Tokens — Variant 1 (Single Source)

> **Status:** DECIDED — Variant 1 is the project color source.
> **Scope:** `dashboard` (React+Vite), `web` (Astro+Starlight), `docs` — all three consume this file, no hard-coded copies.
> **Product:** algorithco guard, CLI `algo`.
> **Source image:** Variant-1 table (light `Och mavzu` / dark `To'q mavzu`), verified 2026-09-20.

## 1. Canonical tokens

| Token | Light | Dark | CSS var | Usage |
|---|---|---|---|---|
| Brand (urg'u) | `#6D4AFF` | `#8E77FF` | `--ag-brand` | Primary actions, links, active states, logo accent |
| Fon (bg) | `#FAFAFB` | `#0E0D15` | `--ag-bg` | App/page background |
| Yuza / Surface (cards) | `#FFFFFF` | `#16151F` | `--ag-surface` | Cards, panels, dialogs |
| Chegara (border) | `#E6E5EE` | `#26243A` | `--ag-border` | Borders, dividers |
| Asosiy matn / logo (text) | `#17161F` | `#F4F3FF` | `--ag-text` | Headings, body, logo |
| Ikkinchi darajali matn (muted) | `#6B6A7B` | `#9C9AB0` | `--ag-text-muted` | Secondary text, placeholders |
| Allow (ruxsat) | `#1E9E63` | `#3DD68C` | `--ag-allow` | `allow` decisions, success only |
| Ask (so'rash) | `#D99A00` | `#F5B82E` | `--ag-ask` | `ask` decisions, warnings only |
| Deny (rad etish) | `#E5484D` | `#FF6B6F` | `--ag-deny` | `deny` decisions, danger only |

Rules:
- Decision colors are semantic and exclusive: `allow`/`ask`/`deny` UI must use only `--ag-allow`/`--ag-ask`/`--ag-deny`. Never reuse them for branding or decoration.
- Brand color is never used for allow/deny semantics.
- Dark theme is not an inversion — use the exact `To'q mavzu` column values.

## 2. CSS variables (copy-paste source)

```css
:root,
[data-theme="light"] {
  --ag-brand: #6D4AFF;
  --ag-bg: #FAFAFB;
  --ag-surface: #FFFFFF;
  --ag-border: #E6E5EE;
  --ag-text: #17161F;
  --ag-text-muted: #6B6A7B;
  --ag-allow: #1E9E63;
  --ag-ask: #D99A00;
  --ag-deny: #E5484D;
}

[data-theme="dark"] {
  --ag-brand: #8E77FF;
  --ag-bg: #0E0D15;
  --ag-surface: #16151F;
  --ag-border: #26243A;
  --ag-text: #F4F3FF;
  --ag-text-muted: #9C9AB0;
  --ag-allow: #3DD68C;
  --ag-ask: #F5B82E;
  --ag-deny: #FF6B6F;
}
```

## 3. Tailwind mapping (dashboard)

Tailwind v4 (`@theme` in CSS):
```css
@theme inline {
  --color-brand: var(--ag-brand);
  --color-bg: var(--ag-bg);
  --color-surface: var(--ag-surface);
  --color-border: var(--ag-border);
  --color-text: var(--ag-text);
  --color-muted: var(--ag-text-muted);
  --color-allow: var(--ag-allow);
  --color-ask: var(--ag-ask);
  --color-deny: var(--ag-deny);
}
```

shadcn/ui mapping: `primary` → `brand`, `background` → `bg`, `card` → `surface`, `border` → `border`, `foreground` → `text`, `muted-foreground` → `text-muted`, `success/warning/destructive` → `allow/ask/deny`.

No other palette may be introduced without an ADR updating this file.

## 4. Astro Starlight (web/docs) mapping

Override Starlight vars in `custom.css`:
```css
:root { --sl-color-accent: var(--ag-brand); }
[data-theme="light"] { --sl-color-accent: #6D4AFF; }
[data-theme="dark"] { --sl-color-accent: #8E77FF; }
```
Page bg/surface/border/text map 1:1 to `--ag-*`. Decision badges (`allow/ask/deny`) reuse dashboard classes.

## 5. Consumers

- `phase-3-dashboard.md` — must import these vars, no local palette.
- `phase-2-enforcement-verifier-loop-edit-web.md` (P2-08 `web`) — Starlight theme must use §4.
- `docs` (privacy/data-flow, reports) — decision examples must use allow/ask/deny tokens.
- `agent/tui` (Phase 4) — closest terminal approximation, same semantics (exact hex where truecolor, else nearest 256-color with ADR note).

## 6. Verification

- [ ] Dashboard light+dark screenshot vs hex picker matches all 9 tokens.
- [ ] `grep -ri '#[0-9a-f]\{6\}' dashboard/src web/src` returns only the 9 light + 9 dark values (via vars, no strays).
- [ ] Contrast check: text/surface and muted/surface pairs recorded (AA for body text; large-text-only exceptions need ADR).
- [ ] `algo why` / history badges: allow/ask/deny map to the three decision tokens in both themes.

## 7. Changes

Any color change = PR to this file + screenshots light/dark + downstream `dashboard`/`web` rebuild. No per-repo token forks.
