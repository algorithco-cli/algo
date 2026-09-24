# @algorithco/guard-contracts

Contracts SDK for algorithco guard (`algorithco_guard.v0`).

> **Status: scaffold only — DO NOT PUBLISH until ADR-0013 accepted + license
> sign-off (ADR-0001) + proto tag pinned.** Repo stays private per ADR-0009 waiver.

- Source: `../../proto/algorithco_guard/v0/*.proto`
- Generated output (`src/gen`, `dist`) is build output — never hand-edit, never commit.
- Consumers pin exact proto tag (`v0.0.1-alpha`): see `../../proto/VERSIONING.md`.
- Build: `npm run build` runs `buf generate` against the pinned tag, then `tsc`.

Publish checklist (all required before `npm publish`):
- [ ] ADR-0013 accepted (scope `@algorithco`, version policy)
- [ ] License SPDX + `LICENSE` file (replaces `UNLICENSED`)
- [ ] Provenance + Trusted Publisher wired in CI
- [ ] `npm pack --dry-run` + `npm publish --dry-run` green
