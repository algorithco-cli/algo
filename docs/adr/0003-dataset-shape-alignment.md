# ADR-0003: Dataset shape alignment (proto adopts RedactionCert object; eval JSON adopts `canonical`)

## Status

draft

Proponent: agent · Date: 2026-09-20

## Context

Audit found proto↔eval drift on `DatasetRecord`:

- Proto (`proto/algorithco_guard/v0/dataset.proto`) declared `string redaction_cert = 8`
  (a certificate id/proof string, REQUIRED, eval-validated).
- Eval's dataset JSON uses a richer `redaction_cert` OBJECT
  `{redacted, scanner, notes}` carrying scanner provenance, and 30 seeds plus
  tests depend on that object shape.
- Eval's dataset JSON also names the pre-tool event field `tool_before`,
  while the proto contract declares it `canonical` (`ToolBefore canonical = 2`).

Tracking: Phase 0 proto contracts (`plans/phase-0-02-proto-contracts-v0.md`);
contracts-first rule (workspace `AGENTS.md` §0.4, `proto/AGENTS.md` §1).
Eval-side rename is handled by a separate eval agent — this ADR covers the
contract decision only; no `eval/` files are touched here.

## Decision

Proto adopts the richer object. Eval JSON adopts the `canonical` naming.

1. Proto adds:
   ```proto
   message RedactionCert {
     bool redacted = 1;
     string scanner = 2;
     string notes = 3;
   }
   ```
   and `DatasetRecord` field 8 becomes `RedactionCert redaction_cert = 8`,
   documented REQUIRED (eval enforces absence → reject).
2. Field numbering is unchanged (`redaction_cert` stays field 8; `canonical`
   stays field 2). No other `DatasetRecord` fields change.
3. Eval dataset JSON renames `tool_before` → `canonical` to match the contract
   (done by the eval agent, out of scope for this proto-side change).

## Alternatives

- **String cert (keep `string redaction_cert = 8`)**: rejected. It loses the
  scanner provenance the 30 seeds and redaction tests rely on, and would force
  eval to flatten or drop `{scanner, notes}` data at the contract boundary.
- **Rename proto field to `tool_before`**: rejected. Contracts-first: `canonical`
  is the declared contract name (`ToolBefore canonical = 2`); the contract does
  not chase implementation naming. Eval aligns to proto, not the reverse.

## Consequences

### Positive

- Single source of truth restored: eval imports `dataset.proto`, no duplicated
  struct, no lossy string↔object mapping at the boundary.
- Scanner provenance (`scanner`, `notes`) is preserved in the contract for
  redaction auditing.
- `canonical` naming is consistent across proto, eval JSON, and docs.

### Negative

- Wire-incompatible change for `redaction_cert` (string → message): any stored
  payloads or consumers using the old string form must migrate. Acceptable in
  Phase 0 pre-tag (no `v*` tag cut yet); the first tag establishes the baseline
  and this change lands before it.
- `buf breaking` against the future baseline will flag this field-type change
  as breaking — expected and intentional for this pre-baseline alignment.

## Verification

- `buf lint` (workdir `proto/`): must exit 0 after the change.
- `buf breaking --against '.git#tag=v0.0.1-alpha'`: expected to fail only
  because tag `v0.0.1-alpha` does not exist yet (tagging happens centrally
  after this change); record the exact error in the PR.
- Contract review: `DatasetRecord.redaction_cert` is type `RedactionCert`,
  field 8, doc-commented REQUIRED; `RedactionCert` carries
  `redacted`/`scanner`/`notes` as specified.
- Eval side (separate agent): 30 seeds use the object shape; eval harness
  rejects records with absent `redaction_cert`; eval JSON uses `canonical`.
