# algo-redact — redact secrets before egress

First Phase 1 task per `docs/adr/0009-p0-gate-waiver.md` + `docs/redact-crate-design.md` (design, not code until waiver).
Gates real user data to Jev: `local-only` default never sends; `redacted` (BYOK, opt-in) sends only `redacted` output.

See `docs/redact-consent-readiness.md` and `docs/privacy-dataflow.md` for the privacy modes and `--show-egress` one-path requirement.
