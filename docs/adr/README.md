# ADRs — algorithco guard

## When an ADR is required

- Any `[DECISION]` item (see [decision-log](../decision-log.md)).
- Any change to latency budgets or eval thresholds.
- Any change to fail-safe behavior (`ask` vs `allow` paths).
- License, distribution, auth, or Windows-scope changes.

## States

- `draft` — proposed, under review on branch `adr/D0X-*`.
- `accepted` — merged to main, implementation may start.
- `superseded` — replaced; link the successor ADR.

CI blocks `D-*` implementation without a merged ADR.

## Process

1. Copy [0000-template](./0000-template.md) to `NNNN-title.md`.
2. Open PR from `adr/D0X-*` with owner + date. CODEOWNERS on `docs/adr/`.
3. Human sign-off required.
4. Update [decision-log](../decision-log.md) status on merge.

## Index

- [0000-template](./0000-template.md)
- [0001-license](./0001-license.md)
- [0002-naming](./0002-naming.md)
- [0004-byok-vs-proxy](./0004-byok-vs-proxy.md) (D-A/D-01, draft 2026-09-20)
- [0005-core-distribution](./0005-core-distribution.md) (D-D, draft 2026-09-20)
- [0006-payments-mor](./0006-payments-mor.md) (D-E/D-08, draft 2026-09-20)
- [0007-windows-scope](./0007-windows-scope.md) (D-F/D-09, draft 2026-09-20)
- [0003-dataset-shape-alignment](./0003-dataset-shape-alignment.md) (draft 2026-09-20)
- [0008-jev-shadow-only](./0008-jev-shadow-only.md) (D-G, draft 2026-09-20)
- [0009-p0-gate-waiver](./0009-p0-gate-waiver.md) (P0 gate waiver for private MVP, draft 2026-09-21 — owner-instructed, countersignature blank)
- [0010-naming-amendment](./0010-naming-amendment.md) (D-10 amendment, draft 2026-09-20, renumbered from 0009 on 2026-09-21 — superseded by ADR-0009 waiver + owner "keep ADR-0002 unchanged")
- [0011-web-stack-pin](./0011-web-stack-pin.md) (web React 18 + Vite 5 pin + upgrade plan, draft 2026-09-22 — private MVP, dep-audit HIGH accepted loopback-only)
- [0012-cel-vs-dsl](./0012-cel-vs-dsl.md) (D-05 policy language, draft 2026-09-23 — spike complete, CEL proposed, human merge required before policy hardens)
