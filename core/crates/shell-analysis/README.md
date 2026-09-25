# algo-shell-analysis — tree-sitter-bash

`parse`, `facts`, `obfuscation` over tree-sitter-bash.

`ParsedCmd{commands:[{bin,args,flags}], redirects[], pipes, subshells, env_assigns, raw}`. On parse error → `Err(ParseFail)`.

`facts()` extracts bins+flags+redirect-targets+net-indicators. Rules match on tree, never raw `contains`.

Obfuscation flags: `base64 -d|base32|xxd|eval $(...)`, `${VAR}` / `${IFS}`, `$'\x..'`, line-continuation, `sh -c` wrapping.
