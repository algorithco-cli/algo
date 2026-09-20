# P1-04 Core: policy + hard-deny + redact (`P1-CORE-3/4`)

## policy (`core/crates/policy/`)

Engine: compile-once at load, `evaluate(facts, profile: strict|balanced|fast) -> Allow|Deny|Abstain + rule_id`. Built-in hard-deny (CODEOWNERS human-reviewed): `rm -rf /|/*`, `mkfs|dd of=/dev/*`, fork-bomb, `curl|wget … | sh|bash`, `chmod 777 /`, `eval+base64 pipe`, `ssh StrictHostKeyChecking=no + rm`, ransomware extensions. Profiles tune Abstain→ask only, never override Deny.

AC: `cargo-mutants` on `deny_list.rs,engine.rs` kill ≥90%; every Deny has rule_id + reason_template; flip-any-deny mutant caught; criterion eval <200µs p50.

## redact (`core/crates/redact/`)

Port gitleaks-style (`regex`+`aho-corasick`): `AKIA…`, `ghp_/gho_`, `xox[bap]-`, `-----BEGIN .*PRIVATE KEY-----`, AWS secret, `AIza`, JWT-ish, `password=|token=`. `redact(text)->(masked, findings[])` stable placeholders `<REDACTED:AWS_KEY>`. Runs before cache logging, Jev payload, audit insert (audit stores `redacted_command` + count, never raw).

AC: proptest no pattern survives + idempotent; <500µs for 10KB; false-positive fixtures (`test`,`example`,`fake`) pass.
