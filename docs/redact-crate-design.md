# Redact crate design — `core/crates/redact` (non-code, for review)

> **Status:** draft — 2026-09-20 (Phase 0, non-code work allowed per gate).
> **First Phase 1 implementation task** (after human review of this plan, per `docs/redact-consent-readiness.md:5`).
> No product code exists yet (`core/.gitkeep` only) — this is a **design doc**, not code.
> All sign-off fields blank — human act.

## 1. Purpose

`core/crates/redact` is the **redact-before-egress** gate for every path that leaves the machine
(`core/crates/provider` Jev `L3`, `backend` audit sync opt-in, and `eval` datasets).
It must be **deterministic, fast, and reviewable** — CODEOWNERS on `core/crates/redact`
per `AGENTS.md:9` + `docs/CODEOWNERS:11-12`.

## 2. Requirements (from `plans/phase-1-04-core-policy-redact.md:9-13` + `docs/redact-consent-readiness.md:2`)

- **Input:** `&str` (command, file snippet, or `redacted_payload` candidate). **Output:** `(masked: String, findings: Vec<Finding>)` where `Finding { kind, span, masked_as }`.
- **Placeholders:** stable, non-reversible `<REDACTED:AWS_KEY>`, `<REDACTED:GITHUB_PAT>`, `<REDACTED:PRIVATE_KEY>`, `<REDACTED:GENERIC_SECRET>`, etc. — never the raw secret.
- **Must mask (gitleaks-style `regex` + `aho-corasick`):** `AKIA…` (20), `aws_secret_access_key` 40-char, `ghp_`/`gho_` (≥20), `xox[baprs]-` (≥10), `-----BEGIN .*PRIVATE KEY-----`, `sk-live`/`sk-test` (≥16), `AIza` (Google), JWT-ish (`eyJ`…), `password=|passwd|pwd|token=|secret=` assignments, high-entropy dense tokens (len≥24, 3+ char classes, entropy>4.5) — see `eval/jev_client/redaction.py:36-58` + `eval/tests/test_no_secrets.py:27-36` (eval stub, port to product).
- **Must NOT mask (false-positive fixtures must pass):** `test`, `example`, `fake`, `placeholder`, `AKIAIOSFODNN7EXAMPLE` (AWS docs example), `sk-test-X7q9…` with `test` prefix, `xoxb-fake` — see plan AC `proptest false-positive fixtures pass`.
- **Performance:** **<500µs for 10KB** (plan AC), zero-alloc hot path where possible (`SmallVec`, `aho-corasick` DFA, no regex on hot path except for long patterns).
- **Idempotence:** `redact(redact(x)) == redact(x)` (proptest).
- **No pattern survives:** `redact(masked).findings == []` and `scan(masked) == []` (proptest + fuzz).
- **One code path:** same function is used for **send** and for `algo log --show-egress` display — no divergence.

## 3. API (proposed, not yet code)

```rust
// core/crates/redact/src/lib.rs (proposed)
pub struct Redactor { /* aho-corasick DFA + regex set, compiled once */ }
impl Redactor {
    pub fn new() -> Self; // compile-once at load, ~few ms
    pub fn redact(&self, input: &str) -> (String, Vec<Finding>);
    pub fn is_redacted(&self, s: &str) -> bool; // scan without alloc
}
#[derive(Debug, Clone, PartialEq)]
pub struct Finding { pub kind: &'static str, pub masked_as: &'static str }
```

- `Redactor::new()` is called once at daemon startup (or `algo` CLI startup) and cloned via `Arc<Redactor>`.
- `provider::JevProvider` holds `Arc<Redactor>` and calls `redact` **synchronously** before any `reqwest` send; if `redact` fails, the path resolves to `ask` / no egress (`proves_ask_on_no_redact`).

## 4. Patterns — taxonomy (eval stub → product, independent of dataset)

| Kind | Regex / AC literal | Placeholder | Source |
|---|---|---|---|
| AWS access key | `AKIA[0-9A-Z]{16}` | `<REDACTED:AWS_KEY>` | `eval/tests/test_no_secrets.py:28` |
| AWS secret | `aws_secret_access_key\s*[:=]\s*['"]?[A-Za-z0-9/+=]{40}` | `<REDACTED:AWS_SECRET>` | same |
| GitHub PAT | `ghp_[A-Za-z0-9]{20,}` / `gho_[A-Za-z0-9]{20,}` | `<REDACTED:GITHUB_PAT>` | same |
| Slack | `xox[baprs]-[A-Za-z0-9-]{10,}` | `<REDACTED:SLACK_TOKEN>` | same |
| PEM | `-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----` | `<REDACTED:PRIVATE_KEY>` | same |
| Vendor SK | `sk-(live|test)-[A-Za-z0-9]{16,}` | `<REDACTED:VENDOR_SK>` | same |
| JWT-ish | `eyJ[A-Za-z0-9_-]+\.eyJ`… (3 segments) | `<REDACTED:JWT>` | plan `phase-1-04` |
| Google | `AIza[A-Za-z0-9_-]{35}` | `<REDACTED:GOOGLE_API_KEY>` | same |
| Password/token assignment | `(?i)\b(password|passwd|pwd|token|secret)\b\s*[:=]\s*['"]?[^'"\s,}]{4,}` | `<REDACTED:CREDENTIAL>` | `eval/jev_client/redaction.py:36` |
| High-entropy dense token | len≥24, 3+ char classes, entropy>4.5 (`eval/tests/test_no_secrets.py:63-73`) | `<REDACTED:HIGH_ENTROPY>` | same |

Aho-corasick literals for fast path: `AKIA`, `ghp_`, `gho_`, `xox`, `BEGIN PRIVATE KEY`, `sk-live`, `sk-test`, `AIza`, `eyJ`.

## 5. Performance budget

- **Hot path:** `aho-corasick` pre-filter (find candidates) → only candidates go to `regex` (avoid regex on clean inputs).
- **Benchmark:** `criterion` bench `redact_10k` (10KB payload, 90% clean + 10% with 1 secret) — **p50 <500µs** per `phase-1-04:13`.
- **Alloc:** `masked` is `String` with `capacity = input.len()`; `findings` is `SmallVec<[Finding; 4]>`.

## 6. Tests (must be green before any real-data egress)

- **Unit:** fixtures for true positives (planted AWS/PEM/vendor-sk/JWT) + false-positive fixtures (`test`/`example`/`fake` → no findings).
- **Property (`proptest`):** `redact(redact(x)) == redact(x)`; `scan(redact(x)) == []`; `redact(x).0` never contains a raw secret from `x` (checked via `scan_text` on masked).
- **Fuzz (`cargo-fuzz` `fuzz_redact`):** `redact` on arbitrary `&[u8]`-as-str (lossy) never panics/OOM; `cargo-fuzz` nightly 1h, PR smoke 60s per `AGENTS.md:52`.
- **Fail-safe:** `proves_ask_on_redact_error` — if `Redactor` is uninitialized or panics, the caller maps to `ask` and does **not** send.

## 7. Integration

- **Provider:** `core/crates/provider::JevProvider` holds `Arc<Redactor>`; `judge()` calls `let (redacted, _findings) = redactor.redact(&event.canonical.redacted_payload?)` — but `canonical.redacted_payload` is **already** redacted at the adapter layer (adapter calls `redact` on parse). Double-redact is idempotent, so egress is safe even if the adapter is bypassed.
- **Audit:** `agent/crates/audit` stores `redacted_command` + `findings.len()` + `redacted` bool, never raw (`plans/phase-1-08:5`).
- **CLI:** `algo log --show-egress` and `algo doctor --show-egress` call the **same** `Redactor::redact` on the same `canonical` payload and print `masked` + `findings` — this is the inspect-what-would-send path (`docs/privacy-dataflow.md:5` Rules).
- **Eval:** `eval/jev_client/redaction.py` + `eval/tests/test_no_secrets.py` are the **spec oracles** for the product crate; they must stay in sync (CI checks `scan_text(redacted) == []`).

## 8. Privacy modes (how redact gates egress)

- `local-only` (default) — `JevProvider::judge` is **not called**; `redact` still runs for local audit/telemetry hygiene, but no network.
- `redacted` (BYOK, explicit opt-in, `ALGO_JEV_API_KEY` env-only) — `redact` → `api.typesafe.ai` (**US**, retention "as long as reasonably necessary", non-US → US transfer — per `docs/privacy-dataflow.md:5` consent draft; `redact-consent-readiness.md:4` blockers).
- `full` (BYOK, second opt-in) — `redact` still runs (defense in depth) but `full` allows unredacted payloads with a second inspect step.

## 9. Gating

Real user commands **must not** be sent to Jev (even `redacted` shadow) until:
all rows in `docs/redact-consent-readiness.md:2` (this crate) + `docs/privacy-dataflow.md` + `blocked-on-typesafe.md:R2` (retention/SLA) are **shipped + human-reviewed** (CODEOWNERS on `core/crates/redact` + `agent/crates/cli-audit` privacy paths) and `docs/exit-gate-P0.md:5` signed.

## 10. Open questions (honest, unresolved)

- Regex vs `aho-corasick` split for JWT/Google patterns (benchmark both).
- High-entropy threshold (len 24, entropy 4.5, 3 classes) — tune on `seed.jsonl` false-positive rate vs product false-negative rate (needs `HUMAN-REVIEW.md` adjudicated ambiguous set).
- `algo log --show-egress` format (JSON vs pretty) — CLI UX review before `agent/crates/cli-audit` hardens.

## Sign-off (leave blank — human act)

- [ ] Redact crate design reviewed: __________ Date: __________
- [ ] `--show-egress` one-path requirement accepted: __________ Date: __________
- [ ] `local-only` default + BYOK `redacted`/`full` opt-in approved: __________ Date: __________
- [ ] Can start `core/crates/redact` implementation after this review: __________ Date: __________
