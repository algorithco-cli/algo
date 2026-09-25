# algo-policy — hard-deny + profiles

Compile-once at load, `evaluate(facts, profile: strict|balanced|fast) -> Allow|Deny|Abstain + rule_id`.

Built-in hard-deny (CODEOWNERS): `rm -rf /|/*`, `mkfs|dd of=/dev/*`, fork-bomb, `curl|wget | sh|bash`, `chmod 777 /`, `eval+base64 pipe`, `ssh StrictHostKeyChecking=no + rm`, ransomware extensions.

Profiles tune Abstain→ask only, never override Deny.

AC: `cargo-mutants` ≥90% on `deny_list.rs,engine.rs`; every Deny has rule_id + reason; criterion eval <200µs p50.
