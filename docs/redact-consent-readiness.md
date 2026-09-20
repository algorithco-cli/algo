# Redact crate + consent/privacy readiness for Phase 1 Jev shadow mode

> **Status:** draft — 2026-09-20 (Phase 0). Do not implement; list gaps only.
> All sign-off fields blank — human review required.

## 1. What Phase 1 shadow needs

Phase 1 shadow sends **redacted** `canonical.redacted_payload` + `tool_kind` + `agent_type`
to Jev (see `docs/privacy-dataflow.md:5-6`, `proto/algorithco_guard/v0/events.proto`,
`eval/jev_client/client.py` redaction hook). Real user commands enter this path
only in `redacted` (default) or `full` (explicit opt-in) mode; `local-only` never
sends. `algo log --show-egress` must show the exact outbound payload before it leaves.

## 2. Redact crate readiness

| Item | Ready? | Evidence / gap | Needed before shadow with real commands |
|---|---|---|---|
| `core/` product code | **No** | `core/README.md:17` — scaffold stub (`.gitkeep` only), no `core/crates/redact` exists yet. Phase 0 rule blocked it. | Implement `core/crates/redact` ported from `eval/jev_client/redaction.py` + `eval/harness` patterns; add `aho-corasick` + regex, stable placeholder `<REDACTED:…>` per `plans/phase-1-04-core-policy-redact.md`. |
| Redaction patterns | **Partial (eval stub)** | `eval/jev_client/redaction.py:36-58` + `eval/tests/test_no_secrets.py:27-36` cover `AKIA`, `ghp_/gho_`, `xox*`, `-----BEGIN .*PRIVATE KEY-----`, `sk-live`, high-entropy — but eval-only, not the product crate, and no `aho-corasick` hot path yet. | Port to product crate behind trait, add `JWT-ish`, `AIza`, `password=|token=` patterns from plan; prove <500µs/10KB, proptest idempotence + no pattern survives. |
| Redact-before-egress wiring | **Gap** | `eval/jev_client/client.py:247-250` does redact-before-POST in the throwaway probe, but `core/crates/provider` Jev provider does not exist yet to own this invariant. | Provider trait (`core/crates/provider`) must enforce redact-before-network at the type level; audit inserts `redacted_command` + count, never raw (plan P1-04 AC). |
| `algo log --show-egress` | **Gap** | No product `algo` CLI yet (`agent/crates/cli-audit` is stub). | Product CLI must render the would-be egress payload from the same redaction used for send. |
| Tests (`proves_ask_on_*`, secret scan) | **Partial** | `eval/tests/test_no_secrets.py:141-159` + harness exists, but no `core` proptest/fuzz for redact completeness/idempotence yet. | `cargo test`, proptest, `cargo-fuzz` shell/policy/redact per `AGENTS.md:52`. |

## 3. Consent / privacy design readiness

| Item | Ready? | Evidence / gap | Needed before real-session shadow |
|---|---|---|---|
| Privacy modes | **Spec exists, not shipped** | `docs/privacy-dataflow.md:5-11` defines `local-only` / `redacted` (default) / `full` (opt-in); `AGENTS.md:68` privacy checklist. | Ship `algo init` privacy prompt (additive merge, inspect step) per `plans/phase-1-08-agent-cli-audit-shadow.md:9`. |
| Consent capture for real commands | **Gap** | `eval/datasets/v0.1/DATASET.md:27-31` plans consent IDs + `source: real-session` provenance, but no consent form, storage, or revocation design exists. | Design: per-workspace opt-in record (consent ID, timestamp, scope, withdraw), stored outside the dataset; every real record carries consent ID + redaction_cert. |
| Real-session collection pipeline | **Gap** | 0 real records (`DATASET.md:37`). No collector exists. | Build a consented collector (adapter → redact → redacted payload only) that the user can inspect and delete; no telemetry that contains source code unless explicitly opted-in. |
| AUP / DPA clearance | **Gap / blocked** | `docs/verify/jev-tos.md:88-93` — AUP URL 404s (dangling MCA ref), DPA VERIFY-OPEN, distillation ticket TBD. | Full AUP read + DPA review + ticket response before any real user traffic goes to Jev. Shadow still sends user data — need clearance. |
| Data residency / retention | **Gap** | `docs/verify/jev-api.md:90-96` — vendor regions unverified, current reports use vantage labels. | Confirm vendor regions/residency; document retention/deletion SLA before Phase 1 shadow with real data. |
| Human review of redacted payloads | **Gap** | No process to sample real redacted payloads for PII/secrets before inclusion. | Manual review queue for the ≥24 real held-out records before `eval-data-v0.2`. |

## 4. Gating

Real user commands **must not** be sent to Jev in any Phase 1 build (even shadow) until:
all rows above are **shipped + human-reviewed** (CODEOWNERS on redaction/privacy paths,
per `AGENTS.md:60` / `docs/CODEOWNERS:11-12`), AUP/DPA cleared, and
`docs/exit-gate-P0.md:5` signed. This report is advisory — it implements nothing.
