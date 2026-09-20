# algo-fingerprint — normalize for cache

`normalize(cmd: &str) -> String` — lowercases argv[0] basename, sorts long flags, replaces paths/hashes/timestamps → `<PATH>/<HASH>/<NUM>`, preserves pipes/redirections/sudo.

`cache_key(normalized, policy_version, profile)` — `blake3(normalize + policy_version + profile)` hex.

See `plans/phase-1-02-core-types-fingerprint.md:9-14` (edges: `VAR=x cmd`, `sudo -u`, `| tee`, quoted, `sh -c` one level).
