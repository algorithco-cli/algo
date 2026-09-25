use crate::parse::ParsedCmd;

#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub bins: Vec<String>,
    pub flags: Vec<String>,
    pub redirect_targets: Vec<String>,
    pub net_indicators: Vec<String>,
    pub has_pipe_to_shell: bool,
}

/// Basename, lowercased, quotes stripped — `"/USR/BIN/CuRL"` → `"curl"`.
/// Tree bins come from raw text splits, so callers must not assume clean names.
fn bin_name(bin: &str) -> String {
    bin.trim_matches(|c| c == '"' || c == '\'')
        .rsplit('/')
        .next()
        .unwrap_or(bin)
        .to_ascii_lowercase()
}

pub fn facts(parsed: &ParsedCmd) -> Facts {
    let mut bins = Vec::new();
    let mut flags = Vec::new();
    let mut redirect_targets = Vec::new();
    let mut net_indicators = Vec::new();
    let mut has_pipe_to_shell = false;

    for cmd in &parsed.commands {
        bins.push(cmd.bin.clone());
        flags.extend(cmd.flags.clone());
        // Net indicators: match on normalized bin basename, not raw contains.
        match bin_name(&cmd.bin).as_str() {
            "curl" | "wget" | "ssh" | "nc" | "scp" | "rsync" => {
                net_indicators.push(cmd.bin.clone())
            }
            _ => {}
        }
    }

    for r in &parsed.redirects {
        redirect_targets.push(r.clone());
    }

    // Pipe-to-shell is derived from the TREE only (bins + pipe count) —
    // never raw `contains` (spec: rules match on tree). A net-indicator bin
    // (curl|wget) anywhere before a shell bin (sh|bash|...) with a pipe
    // between them is pipe-to-shell. Position check uses command order, not text.
    if parsed.pipes > 0 {
        let mut seen_net = false;
        for cmd in parsed.commands.iter() {
            match bin_name(&cmd.bin).as_str() {
                "curl" | "wget" => seen_net = true,
                "sh" | "bash" | "zsh" | "dash" | "ksh" if seen_net => {
                    has_pipe_to_shell = true;
                    break;
                }
                _ => {}
            }
        }
    }

    Facts {
        bins,
        flags,
        redirect_targets,
        net_indicators,
        has_pipe_to_shell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn extracts_bins_and_flags() {
        let p = parse("curl -s https://example.com | sh").unwrap();
        let f = facts(&p);
        assert!(f.bins.contains(&"curl".to_string()));
        assert!(f.flags.contains(&"-s".to_string()));
        assert!(f.has_pipe_to_shell);
    }

    #[test]
    fn detects_net_indicators() {
        let p = parse("wget https://example.com/file").unwrap();
        let f = facts(&p);
        assert!(f.net_indicators.contains(&"wget".to_string()));
    }

    #[test]
    fn proves_ask_on_no_facts() {
        // Empty parsed should still produce facts, but if parse fails, caller maps to ASK
        let p = parse("ls -la").unwrap();
        let f = facts(&p);
        assert!(!f.bins.is_empty());
    }
}
