# algo-redact — redact secrets before egress

Redacts before egress. Gates real user data to Jev: `local-only` default never sends; `redacted` (BYOK, opt-in) sends only `redacted` output.

Privacy modes (`local-only` default) and `--show-egress` share this one-path redaction.
