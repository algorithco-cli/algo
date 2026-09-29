# algocli

Contracts SDK for algorithco guard (`algorithco_guard.v0`). CLI stays `algo`; this is the registry name.

> **Status: scaffold only — DO NOT PUBLISH until scope (`@algorithco`, version policy)
> is decided + proto tag pinned.** Repo stays private until release.

- Source: exact Git tag `v0.0.1-alpha` (override only with `ALGO_PROTO_TAG`)
- Generated output (`src/gen`, `dist`) is build output — never hand-edit, never commit.
- Consumers pin exact proto tag (`v0.0.1-alpha`): see `../../proto/VERSIONING.md`.
- Build: `npm run build` exports the pinned tag into an isolated temporary tree,
  runs `buf generate` there, then runs `tsc`.

Publish checklist (all required before `npm publish`):
- [ ] Scope decided (`@algorithco`, version policy)
- [x] License SPDX + `LICENSE` file (GPL-3.0-only, owner decision 2026-09-27, final)
- [x] Provenance + Trusted Publisher wired in CI
- [x] `npm pack --dry-run` + `npm publish --dry-run` green (`import_extension=js` for Node ESM)
