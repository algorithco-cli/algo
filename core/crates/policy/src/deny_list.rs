//! Hard deny list — CODEOWNERS human-reviewed (§0.9).
//! Every Deny has rule_id + reason_template; flip-any-deny mutant must be caught.

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub pattern: Regex,
    pub reason: &'static str,
}

fn r(pat: &str) -> Regex {
    Regex::new(pat).unwrap()
}

static RULES: OnceLock<Vec<Rule>> = OnceLock::new();

pub fn hard_deny_rules() -> &'static [Rule] {
    RULES.get_or_init(|| {
        vec![
            Rule {
                id: "DENY_RM_RF_ROOT",
                // Flags in any order: -rf, -fr, -Rf (capital R == recursive), -r -f (split),
                // --recursive+--force. Target must be exactly `/` (delimiter after) —
                // `/tmp/x` must NOT match. Delimiters include `)`/backtick/braces so
                // `$(rm -rf /)`, backtick and `${...}` wrappings can't dodge (2026-09-23 deep battery).
                pattern: r(r#"\brm\b[^|;]*?(?:-[a-zA-Z]*[rR][a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*[rR][a-zA-Z]*|-[rR]\b[^|;]*?-f\b|-f\b[^|;]*?-[rR]\b|--recursive\b[^|;]*?--force\b|--force\b[^|;]*?--recursive\b)\b[^|;]*?\s/(?:\s|$|;|&|"|'|/|\(|\)|`|\{|\})"#),
                reason: "hard deny: rm -rf / (rule DENY_RM_RF_ROOT)",
            },
            Rule {
                id: "DENY_RM_RF_ALL",
                pattern: r(r#"\brm\b[^|;]*?(?:-[a-zA-Z]*[rR][a-zA-Z]*f[a-zA-Z]*|-[a-zA-Z]*f[a-zA-Z]*[rR][a-zA-Z]*|-[rR]\b[^|;]*?-f\b|-f\b[^|;]*?-[rR]\b|--recursive\b[^|;]*?--force\b|--force\b[^|;]*?--recursive\b)\b[^|;]*?\s/\*(?:\s|$|;|&|"|'|/|\(|\)|`|\{|\})"#),
                reason: "hard deny: rm -rf /* (rule DENY_RM_RF_ALL)",
            },
            Rule {
                id: "DENY_MKFS",
                pattern: r(r"\bmkfs(?:\.[a-z0-9]+)?\b"),
                reason: "hard deny: mkfs (rule DENY_MKFS)",
            },
            Rule {
                id: "DENY_DD_DEV",
                // `of=` targeting a block device. Device allowlist covers sd/hd/vd/xvd,
                // nvme, mmcblk, loop, dm, md. Deliberately NOT matching of=/dev/null|zero|urandom
                // (harmless discard sources) — blanket /dev/ would over-block.
                pattern: r(r"\bdd\b[^|;]*\bof\s*=\s*/dev/(?:sd[a-z]+|hd[a-z]+|vd[a-z]+|xvd[a-z]+|nvme[0-9]+n[0-9]+[a-z0-9]*|mmcblk[0-9]+(?:p[0-9]+)?|loop[0-9]+|dm-[0-9]+|md[0-9]+|sda|sdb|hda|vda)[0-9a-z]*\b"),
                reason: "hard deny: dd of=/dev/* (rule DENY_DD_DEV)",
            },
            Rule {
                id: "DENY_FORK_BOMB",
                pattern: r(r":\(\)\s*\{"),
                reason: "hard deny: fork-bomb :(){:|:&};: (rule DENY_FORK_BOMB)",
            },
            Rule {
                id: "DENY_CURL_PIPE_SH",
                // Shell may be a bare bin or a multi-segment path (`| /bin/sh`, `| /usr/bin/bash`).
                // `\b` after the shell bin keeps `| shuf` / `| show` safe.
                pattern: r(r"\b(?:curl|wget)\b[^|]*\|\s*(?:[A-Za-z0-9_.+:/-]*/)?(?:sh|bash|zsh|dash|ksh)\b"),
                reason: "hard deny: curl|wget | sh (rule DENY_CURL_PIPE_SH)",
            },
            Rule {
                id: "DENY_CHMOD_777_ROOT",
                // Flags may precede or follow the mode (`chmod 777 -R /`, `chmod -R 777 /`).
                // Target must be `/` — `chmod 777 file` must NOT match.
                // Same substitution-proof delimiters as the rm rules above.
                pattern: r(r#"\bchmod\b[^|;]*777[^|;]*\s/(?:\s|$|;|&|"|'|/|\(|\)|`|\{|\})"#),
                reason: "hard deny: chmod 777 / (rule DENY_CHMOD_777_ROOT)",
            },
            Rule {
                id: "DENY_CHMOD_777_RECURSIVE",
                pattern: r(r"\bchmod\s+-R\s+777\b"),
                reason: "hard deny: chmod -R 777 (rule DENY_CHMOD_777_RECURSIVE)",
            },
            Rule {
                id: "DENY_EVAL_BASE64",
                pattern: r(r"\beval\b[^|;]*\$\(\s*echo\s+[^|]*\|\s*base64\s+-d"),
                reason: "hard deny: eval + base64 pipe (rule DENY_EVAL_BASE64)",
            },
            Rule {
                id: "DENY_BASE64_PIPE_SH",
                pattern: r(r"\bbase64\s+-d\b[^|]*\|\s*(?:[A-Za-z0-9_.+:/-]*/)?(?:sh|bash|eval)\b"),
                reason: "hard deny: base64 | sh (rule DENY_BASE64_PIPE_SH)",
            },
            Rule {
                id: "DENY_SSH_BYPASS_RM",
                pattern: r(r"\bssh\b[^|;]*StrictHostKeyChecking\s*=\s*no\b[^|;]*\brm\b"),
                reason: "hard deny: ssh StrictHostKeyChecking=no + rm (rule DENY_SSH_BYPASS_RM)",
            },
            Rule {
                id: "DENY_NC_E",
                // `-e` may point at an absolute shell or a PATH-resolved `sh`.
                pattern: r(r"\bnc\b[^|;]*-e\s*(?:[A-Za-z0-9_.+:/-]*/)?(?:sh|bash)\b"),
                reason: "hard deny: nc -e /bin/sh (rule DENY_NC_E)",
            },
            Rule {
                id: "DENY_RANSOMWARE_EXT",
                pattern: r(r"\.(?:encrypted|locked|crypt|ransom)\b"),
                reason: "hard deny: ransomware extension (rule DENY_RANSOMWARE_EXT)",
            },
        ]
    })
}

/// For `cargo-mutants` — every rule must have an id.
pub const HARD_DENY_RULES: &[&str] = &[
    "DENY_RM_RF_ROOT",
    "DENY_RM_RF_ALL",
    "DENY_MKFS",
    "DENY_DD_DEV",
    "DENY_FORK_BOMB",
    "DENY_CURL_PIPE_SH",
    "DENY_CHMOD_777_ROOT",
    "DENY_CHMOD_777_RECURSIVE",
    "DENY_EVAL_BASE64",
    "DENY_BASE64_PIPE_SH",
    "DENY_SSH_BYPASS_RM",
    "DENY_NC_E",
    "DENY_RANSOMWARE_EXT",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deny_has_rule_id() {
        for rule in hard_deny_rules() {
            assert!(!rule.id.is_empty());
            assert!(HARD_DENY_RULES.contains(&rule.id));
            assert!(!rule.reason.is_empty());
        }
    }

    #[test]
    fn proves_ask_on_no_match() {
        // No hard deny should match safe commands
        let safe = "ls -la";
        for rule in hard_deny_rules() {
            assert!(
                !rule.pattern.is_match(safe),
                "rule {} matched safe",
                rule.id
            );
        }
    }
}
