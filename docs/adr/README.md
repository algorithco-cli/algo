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
