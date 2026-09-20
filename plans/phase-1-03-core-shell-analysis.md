# P1-03 Core: shell-analysis (`P1-CORE-2`)

`core/crates/shell-analysis/src/{lib,parse,facts,obfuscation}.rs`

## Parse

tree-sitter-bash → `ParsedCmd{commands:[{bin,args,flags}], redirects[], pipes, subshells, env_assigns, raw}`. On parse error → `Err(ParseFail)` (caller maps to ask).

## facts()

bins+flags+redirect-targets+net-indicators (`curl|wget|ssh|nc`). Rules match on tree, never raw `contains`.

## obfuscation flags

`base64 -d|base32|xxd|eval $(...)`, `${VAR}` / `${IFS}`, `$'\x..'`, line-continuation splits, `sh -c` wrapping.

## Acceptance

- `rm -rf /`, `mkfs.*`, `:(){:|:&};:`, `curl … | sh`, `dd of=/dev/sda`, `chmod -R 777 /`, `eval $(echo cm0gLXJmIC8=|base64 -d)` → dangerous:true.
- `ls -la`, `cargo test`, `git status` → safe.
- `tests/regression_{safe,dangerous,obfuscated}.json`; `cargo-fuzz` `fuzz_parse` 1h no panic/OOM nightly; PR smoke 60s.
