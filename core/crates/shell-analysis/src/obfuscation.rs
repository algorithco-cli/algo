use crate::parse::ParsedCmd;

#[derive(Debug, Clone, PartialEq)]
pub struct ObfuscationFlags {
    pub has_base64: bool,
    pub has_base32: bool,
    pub has_xxd: bool,
    pub has_eval_subshell: bool,
    pub has_var_expansion: bool,
    pub has_ifs: bool,
    pub has_hex_escape: bool,
    pub has_line_continuation: bool,
    pub has_sh_c_wrapping: bool,
}

pub fn obfuscation_flags(parsed: &ParsedCmd) -> ObfuscationFlags {
    let raw = &parsed.raw;
    ObfuscationFlags {
        has_base64: raw.contains("base64") && (raw.contains("-d") || raw.contains("--decode")),
        has_base32: raw.contains("base32"),
        has_xxd: raw.contains("xxd"),
        has_eval_subshell: raw.contains("eval") && raw.contains("$("),
        has_var_expansion: raw.contains("${VAR}") || raw.contains("${IFS}") || raw.contains("${"),
        has_ifs: raw.contains("${IFS}"),
        has_hex_escape: raw.contains("$'\\x") || raw.contains("$\"\\x"),
        has_line_continuation: raw.contains("\\\n") || raw.contains("\\\r\n"),
        has_sh_c_wrapping: raw.contains("sh -c") || raw.contains("bash -c"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn detects_base64() {
        let p = parse("echo cm0gLXJmIC8= | base64 -d | sh").unwrap();
        let f = obfuscation_flags(&p);
        assert!(f.has_base64);
    }

    #[test]
    fn detects_ifs() {
        let p = parse("echo ${IFS}test").unwrap();
        let f = obfuscation_flags(&p);
        assert!(f.has_ifs);
        assert!(f.has_var_expansion);
    }

    #[test]
    fn detects_sh_c() {
        let p = parse("sh -c 'ls -la'").unwrap();
        let f = obfuscation_flags(&p);
        assert!(f.has_sh_c_wrapping);
    }

    #[test]
    fn detects_eval_subshell() {
        let p = parse("eval $(echo hi)").unwrap();
        let f = obfuscation_flags(&p);
        assert!(f.has_eval_subshell);
    }
}
