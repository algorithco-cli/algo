# ADRs — algorithco guard

## When an ADR is required

- Any change to latency budgets or eval thresholds.
- Any change to fail-safe behavior (`ask` vs `allow` paths).
- License, distribution, auth, or Windows-scope changes.

## States

- `draft` — proposed, under review on branch `adr/D0X-*`.
- `accepted` — merged to main, implementation may start.
- `superseded` — replaced; link the successor ADR.

## Process

1. Copy [0000-template](./0000-template.md) to `NNNN-title.md`.
2. Open PR from `adr/D0X-*` with owner + date. CODEOWNERS on `docs/adr/`.
3. Human sign-off required.

## Index

- [0000-template](./0000-template.md)
- [0002-naming](./0002-naming.md)
- [0003-dataset-shape-alignment](./0003-dataset-shape-alignment.md) (draft 2026-09-20)
- [0011-web-stack-pin](./0011-web-stack-pin.md) (web React 18 + Vite 5 pin + upgrade plan, draft 2026-09-22 — private MVP, dep-audit HIGH accepted loopback-only)
