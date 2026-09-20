# VERSIONING — proto contracts (`algorithco_guard.v0`)

## Scheme

- Semantic versioning (`MAJOR.MINOR.PATCH`) on git tags, e.g. `v0.0.1-alpha`.
- Phase 0 recommendation: **`v0.0.1-alpha`** (first alpha tag; see `CHANGELOG.md`).
- Proto package stays `algorithco_guard.v0` until an approved major bump.
  A `v1` package requires an ADR + migration note — never rename silently.

## Generate at build, never hand-edit

- All language output (`gen/rust`, `gen/ts`) is **build output**.
- Consumers regenerate from the pinned tag at build time (`buf generate`).
- **Never hand-edit generated code.** Fix the `.proto` source, bump the
  version, re-tag, and regenerate.
- CI verifies this: generated output is either git-ignored or checked with
  `buf generate --template buf.gen.yaml` producing a clean diff.

## Exact pins

- Consumers pin the **exact tag**, never a branch or `main`:
  - Rust: `algo-*` crates depend on generated code from tag `vX.Y.Z`.
  - TS: `npm`/dashboard builds run `buf export` / `buf generate` against tag `vX.Y.Z`.
  - Eval harness imports `dataset.proto` from the same tag — no duplicated struct.
- Example pin (after tagging):
  - `buf export --against '.git#tag=v0.0.1-alpha'`
  - `buf generate --template buf.gen.yaml` at the pinned commit.

## Breaking-change rule

- Every PR runs `buf lint` and `buf breaking --against '.git#tag=<latest>'`.
- A demo breaking PR **must fail** `breaking` (acceptance `P0-PROTO-1`).
- `FILE`-level breaking policy (see `buf.yaml`); wire stubs under
  `EXPERIMENTAL` header are exempt from stability promises until promoted.

## Tag procedure

1. `buf lint` green.
2. `buf breaking --against '.git#tag=<latest>'` green (or intentional major).
3. Update `CHANGELOG.md` (Unreleased → version + date).
4. Tag: `git tag v0.0.1-alpha && git push origin v0.0.1-alpha`.
5. CI codegen smoke: Rust `cargo check` + TS `npm run build` from the tag.
