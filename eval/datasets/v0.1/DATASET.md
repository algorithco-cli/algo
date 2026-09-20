# Dataset v0.1 — source, version, redaction certificate, license

- **Version:** v0.1 (seed slice: 30 records; full 200–300 collection lands in `P0-EVAL-3`)
- **Source:** hand-authored synthetic seed commands modeling shell-heavy agent
  tool use, plus a small edit/write sample. No production logs, no user code,
  no real credentials — every payload is invented for the eval.
- **Labeling:** per `eval/labeling-guide.md` (SAFE / DANGEROUS / AMBIGUOUS +
  obfuscation appendix). Seed labels: **12 safe / 10 dangerous / 8 ambiguous**;
  **12/30 (40%) obfuscated**, covering all five non-NONE tags.
- **Kappa pilot:** pending full collection — 20-record double-label with
  kappa ≥ 0.7 required before tagging `eval-data-v0.1` (procedure in
  `labeling-guide.md` §5).
- **Redaction certificate:** every record carries
  `redaction_cert: {redacted: true, scanner: "eval-no-secrets-v0.1", notes}`.
  CI (`tests/test_no_secrets.py`) scans all payloads for AWS/PEM/vendor-sk /
  high-entropy shapes and fails the build on any hit or missing certificate.
- **Schema:** `schema.json` mirrors `proto/dataset.proto` `DatasetRecord`
  (record_id, canonical ToolBefore, label, rationale, obfuscation, annotator,
  dataset_version, redaction_cert). The proto is the contract; this JSON
  schema is a CI validation convenience, not a second source of truth.
- **License:** TBD — legal sign-off required before any external release
  (see decision D-07). Treat as internal-only until then.
- **Tag:** full-collection release will be tagged `eval-data-v0.1`.
