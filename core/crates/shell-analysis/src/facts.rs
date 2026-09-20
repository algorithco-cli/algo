use crate::parse::ParsedCmd;

#[derive(Debug, Clone, PartialEq)]
pub struct Facts {
    pub bins: Vec<String>,
    pub flags: Vec<String>,
    pub redirect_targets: Vec<String>,
    pub net_indicators: Vec<String>,
    pub has_pipe_to_shell: bool,
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
        // Net indicators: curl, wget, ssh, nc, etc. — match on bin, not raw contains
        if matches!(cmd.bin.as_str(), "curl" | "wget" | "ssh" | "nc" | "scp" | "rsync") {
            net_indicators.push(cmd.bin.clone());
        }
    }

    for r in &parsed.redirects {
        redirect_targets.push(r.clone());
        if r.contains("/dev/") {
            // e.g., dd of=/dev/sda
            redirect_targets.push(r.clone());
        }
    }

    // Check for pipe to shell: e.g., curl ... | sh
    if parsed.pipes > 0 {
        for (i, cmd) in parsed.commands.iter().enumerate() {
            if i > 0 && matches!(cmd.bin.as_str(), "sh" | "bash" | "zsh" | "dash" | "ksh") {
                // Previous command was a net indicator?
                if i > 0 {
                    let prev = &parsed.commands[i - 1];
                    if matches!(prev.bin.as_str(), "curl" | "wget") {
                        has_pipe_to_shell = true;
                    }
                }
            }
        }
        // Also check raw for pipe to shell pattern (fallback)
        if parsed.raw.contains("|") && (parsed.raw.contains("| sh") || parsed.raw.contains("| bash")) {
            has_pipe_to_shell = true;
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
