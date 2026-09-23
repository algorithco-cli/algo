//! Shared fact shape for the P1-01 spike. Spike-only synthetic facts —
//! NOT the product `tree-sitter-bash` parser. Both engines consume this.

#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// argv[0] basenames in pipeline order, e.g. ["curl", "sh"]
    pub bins: Vec<String>,
    /// whitespace tokens after argv[0], e.g. ["-rf", "/"]
    pub flags: Vec<String>,
    /// ALL whitespace tokens, lowercased (for mid-line keywords like `rm`
    /// after `ssh ... StrictHostKeyChecking=no`)
    pub toks: Vec<String>,
    /// pipe into sh/bash/zsh/dash/ksh detected
    pub has_pipe_to_shell: bool,
    /// curl|wget|ssh|nc present
    pub net: bool,
    /// raw command string (for `contains` / `matches`)
    pub raw: String,
}

fn is_shell(s: &str) -> bool {
    matches!(s, "sh" | "bash" | "zsh" | "dash" | "ksh")
}

/// Naive whitespace tokenizer, one level of `|` splitting. Documented
/// limitation: no quoting/expansion handling (product uses tree-sitter).
pub fn facts_of(cmd: &str) -> Facts {
    let stages: Vec<&str> = cmd.split('|').collect();
    let mut bins = Vec::new();
    for stage in &stages {
        let toks: Vec<&str> = stage.split_whitespace().collect();
        if let Some(first) = toks.first() {
            let base = first.rsplit('/').next().unwrap_or(first);
            // strip VAR=x assignments
            if !base.contains('=') {
                bins.push(base.to_ascii_lowercase());
            }
        }
    }
    let mut flags = Vec::new();
    for tok in cmd.split_whitespace().skip(1) {
        flags.push(tok.to_string());
    }
    let toks: Vec<String> = cmd
        .split_whitespace()
        .map(|t| t.to_ascii_lowercase())
        .collect();
    let has_pipe_to_shell = stages.len() > 1
        && stages[1..]
            .iter()
            .any(|s| s.split_whitespace().any(is_shell));
    let net = bins
        .iter()
        .any(|b| matches!(b.as_str(), "curl" | "wget" | "ssh" | "nc"));
    Facts {
        bins,
        flags,
        toks,
        has_pipe_to_shell,
        net,
        raw: cmd.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizer_basics() {
        let f = facts_of("curl -s https://example.com | sh");
        assert_eq!(f.bins, vec!["curl", "sh"]);
        assert!(f.has_pipe_to_shell);
        assert!(f.net);
    }
}
