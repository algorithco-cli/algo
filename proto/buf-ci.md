# buf CI (Phase 0 proto v0)

Real workflow lives at `.github/workflows/buf.yml` (workspace root).
This file documents intent so the contract is reviewable without opening YAML.

## Jobs

1. **`buf lint`**
   - Runs on every PR touching `proto/**`.
   - Command: `buf lint proto`
   - Acceptance `P0-PROTO-1`: CI green required for merge.
2. **`buf breaking`**
   - Runs on every PR touching `proto/**`.
   - Command: `buf breaking proto --against '.git#tag=<latest>'`
     (CI resolves `<latest>` to the most recent `v*` tag, e.g. `v0.0.1-alpha`).
   - A demo breaking PR **must fail** this job (proves the gate works).
3. **Codegen smoke (`P0-PROTO-6`)**
   - `buf generate proto --template proto/buf.gen.yaml`
   - Rust scratch-consumer: `cargo check`
   - TS scratch-consumer: `npm run build`
   - Runs on tag push (`v*`) and on PRs that change codegen inputs.

## Tooling

- `buf` version is pinned in the workflow (see `buf.yml`).
- Plugins: `protoc-gen-prost` (Rust) + `protoc-gen-es` (TS).
  Versions pinned in CI env; `buf.lock` tracks remote deps (currently none).
- Never invent Jev APIs: no Jev service/method stubs in CI or protos.

## Local repro

```sh
buf lint proto
buf breaking proto --against '.git#tag=v0.0.1-alpha'
buf generate proto --template proto/buf.gen.yaml
```
