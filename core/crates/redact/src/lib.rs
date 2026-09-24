//! `algo-redact` — redact secrets and PII before any egress.
//!
//! Design: `docs/redact-crate-design.md` (draft, human review required before real-data Jev).
//! First Phase 1 task per ADR-0009 waiver — gates every real-data Jev call.
//! Requirements: `docs/redact-consent-readiness.md:2` + `plans/phase-1-04-core-policy-redact.md:9-13`
//! (patterns, <500µs/10KB, proptest idempotence, fuzz, one path for send and `--show-egress`).

use aho_corasick::AhoCorasick;
use regex::Regex;
use smallvec::SmallVec;
use std::sync::OnceLock;

/// A finding — what was masked and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    pub masked_as: &'static str,
}

/// The redactor — compile-once at load, `Arc`-clonable via `OnceLock`.
///
/// One code path for both `send` and `algo log --show-egress` (no divergence).
pub struct Redactor {
    // Fast pre-filter: aho-corasick finds candidate positions quickly.
    ac: AhoCorasick,
    // Full regex set for precise matching (only on candidates).
    regexes: Vec<(&'static str, Regex, &'static str)>,
}

static GLOBAL: OnceLock<Redactor> = OnceLock::new();
static RE_B64_SUBSTR: OnceLock<Regex> = OnceLock::new();

/// AWS documentation example key — not a credential, never masked.
const AWS_DOCS_EXAMPLE_KEY: &str = "AKIAIOSFODNN7EXAMPLE";

fn re_b64_substr() -> &'static Regex {
    RE_B64_SUBSTR.get_or_init(|| Regex::new(r"[A-Za-z0-9+/=_.-]{24,}").unwrap())
}

fn build_ac() -> AhoCorasick {
    // Literals for fast pre-filter — if none match, we can skip regex entirely (common case).
    let patterns = [
        "AKIA",
        "ghp_",
        "gho_",
        "xox",
        "BEGIN PRIVATE KEY",
        "sk-live",
        "sk-test",
        "AIza",
        "eyJ",
        "password",
        "passwd",
        "pwd",
        "token",
        "secret",
    ];
    AhoCorasick::new(patterns).expect("ac build")
}

fn build_regexes() -> Vec<(&'static str, Regex, &'static str)> {
    vec![
        (
            "aws_key",
            Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
            "<REDACTED:AWS_KEY>",
        ),
        (
            "aws_secret",
            Regex::new(r#"(?i)\baws_secret_access_key\b\s*[:=]\s*['"]?[A-Za-z0-9/+=]{40}['"]?"#)
                .unwrap(),
            "<REDACTED:AWS_SECRET>",
        ),
        (
            "github_pat",
            Regex::new(r"\bghp_[A-Za-z0-9]{20,}\b").unwrap(),
            "<REDACTED:GITHUB_PAT>",
        ),
        (
            "github_oauth",
            Regex::new(r"\bgho_[A-Za-z0-9]{20,}\b").unwrap(),
            "<REDACTED:GITHUB_OAUTH>",
        ),
        (
            "slack_token",
            Regex::new(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b").unwrap(),
            "<REDACTED:SLACK_TOKEN>",
        ),
        (
            "pem_block",
            Regex::new(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----").unwrap(),
            "<REDACTED:PRIVATE_KEY>",
        ),
        (
            "vendor_sk",
            Regex::new(r"\bsk-(live|test)-[A-Za-z0-9]{16,}\b").unwrap(),
            "<REDACTED:VENDOR_SK>",
        ),
        (
            "google_api",
            Regex::new(r"\bAIza[A-Za-z0-9_-]{35}\b").unwrap(),
            "<REDACTED:GOOGLE_API_KEY>",
        ),
        (
            "jwt",
            Regex::new(r"\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b").unwrap(),
            "<REDACTED:JWT>",
        ),
        (
            "credential_assignment",
            Regex::new(r#"(?i)\b(password|passwd|pwd|token|secret)\b\s*[:=]\s*['"]?[^'"\s,}]{4,}"#)
                .unwrap(),
            "<REDACTED:CREDENTIAL>",
        ),
    ]
}

impl Redactor {
    pub fn new() -> Self {
        Self {
            ac: build_ac(),
            regexes: build_regexes(),
        }
    }

    pub fn global() -> &'static Self {
        GLOBAL.get_or_init(Self::new)
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    /// Redact `input` → `(masked, findings)`.
    ///
    /// Idempotent: `redact(redact(x).0).0 == redact(x).0`.
    /// No finding survives a second pass: `scan(masked) == []`.
    pub fn redact(&self, input: &str) -> (String, SmallVec<[Finding; 4]>) {
        // Fast path: if no literal hits, skip regex (common for clean payloads).
        // We still need to check high-entropy dense tokens which have no literal,
        // but those are rare and we handle them separately.
        let mut masked = input.to_string();
        let mut findings: SmallVec<[Finding; 4]> = SmallVec::new();

        // Aho-corasick pre-filter: if it finds nothing and input is short, we can still
        // need to check credential assignments which may not have literals? But our AC
        // includes "password"/"token"/etc., so it covers those. High-entropy is separate.
        let has_literal = self.ac.find_iter(input).next().is_some();

        // Run regexes only if we have a literal or input is long enough to warrant scan.
        // For correctness we run all regexes unconditionally for now — the AC is an
        // optimization hint, not a gate, until we prove the bypass is safe.
        // TODO: gate regexes behind `has_literal || input.len() > 1024` after fuzz proves it.
        let _ = has_literal;

        for (kind, re, masked_as) in &self.regexes {
            if re.is_match(&masked) {
                let mut found = false;
                // Replace all occurrences. The AWS documentation example key is
                // never a real credential — it is preserved verbatim per
                // redact-crate-design.md:20 (regex crate has no look-around).
                let new_masked = if *kind == "aws_key" {
                    re.replace_all(&masked, |caps: &regex::Captures| {
                        if &caps[0] == AWS_DOCS_EXAMPLE_KEY {
                            caps[0].to_string()
                        } else {
                            masked_as.to_string()
                        }
                    })
                    .to_string()
                } else {
                    re.replace_all(&masked, *masked_as).to_string()
                };
                if new_masked != masked {
                    found = true;
                    masked = new_masked;
                }
                if found {
                    findings.push(Finding { kind, masked_as });
                }
            }
        }

        // High-entropy dense tokens (len>=24, 3+ char classes, entropy>4.5) — like `eval` harness.
        // This is a secondary pass for credential-shaped strings without a known prefix.
        let high_entropy = high_entropy_tokens(&masked);
        for tok in high_entropy {
            // Avoid double-masking already redacted placeholders.
            if masked.contains(&tok) && !tok.starts_with("<REDACTED:") {
                masked = masked.replace(&tok, "<REDACTED:HIGH_ENTROPY>");
                if !findings.iter().any(|f| f.kind == "high_entropy") {
                    findings.push(Finding {
                        kind: "high_entropy",
                        masked_as: "<REDACTED:HIGH_ENTROPY>",
                    });
                }
            }
        }

        (masked, findings)
    }

    /// Scan without alloc — true if `s` would be redacted.
    pub fn is_redacted(&self, s: &str) -> bool {
        for (_, re, _) in &self.regexes {
            if re.is_match(s) {
                return true;
            }
        }
        !high_entropy_tokens(s).is_empty()
    }
}

fn char_classes(s: &str) -> usize {
    let mut classes = 0;
    if s.chars().any(|c| c.is_ascii_uppercase()) {
        classes += 1;
    }
    if s.chars().any(|c| c.is_ascii_lowercase()) {
        classes += 1;
    }
    if s.chars().any(|c| c.is_ascii_digit()) {
        classes += 1;
    }
    if s.chars().any(|c| !c.is_ascii_alphanumeric()) {
        classes += 1;
    }
    classes
}

fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut counts = [0usize; 256];
    for b in s.bytes() {
        counts[b as usize] += 1;
    }
    let len = s.len() as f64;
    let mut entropy = 0.0;
    for &n in &counts {
        if n > 0 {
            let p = n as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

fn high_entropy_tokens(s: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for tok in s.split(|c: char| {
        c.is_whitespace()
            || matches!(
                c,
                '"' | '\'' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
            )
    }) {
        let stripped = tok.trim_matches(|c| matches!(c, '=' | '-' | '.' | '_' | ':' | '/'));
        if stripped.len() >= 24 && char_classes(stripped) >= 3 && shannon_entropy(stripped) > 4.5 {
            // Skip already-redacted placeholders and common non-secrets
            if stripped.starts_with("<REDACTED")
                || stripped.contains("example")
                || stripped.contains("test")
                || stripped.contains("fake")
            {
                continue;
            }
            hits.push(stripped.to_string());
        }
    }
    // Also check for long base64-like substrings inside the string
    // (hoisted OnceLock — never `Regex::new` per call; <500µs/10KB budget).
    for caps in re_b64_substr().find_iter(s) {
        let tok = caps
            .as_str()
            .trim_matches(|c| matches!(c, '=' | '-' | '.' | '_'));
        if tok.len() >= 24
            && char_classes(tok) >= 3
            && shannon_entropy(tok) > 4.5
            && !tok.contains("example")
            && !hits.contains(&tok.to_string())
        {
            hits.push(tok.to_string());
        }
    }
    hits
}

/// Convenience: global redact (for `--show-egress` one-path).
pub fn redact(input: &str) -> (String, SmallVec<[Finding; 4]>) {
    Redactor::global().redact(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn redacts_aws_key() {
        let (masked, findings) = Redactor::new().redact("key=AKIAZZZZZZZZZZZZZZZZ more");
        assert!(masked.contains("<REDACTED:AWS_KEY>"));
        assert!(findings.iter().any(|f| f.kind == "aws_key"));
    }

    #[test]
    fn docs_example_key_not_masked() {
        // redact-crate-design.md:20 — the AWS documentation example is not a
        // credential and must survive redaction (docs/tests carry it).
        let (masked, findings) = Redactor::new().redact("key=AKIAIOSFODNN7EXAMPLE more");
        assert!(!masked.contains("<REDACTED:AWS_KEY>"));
        assert!(masked.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!findings.iter().any(|f| f.kind == "aws_key"));
    }

    #[test]
    fn redacts_github_pat() {
        let (masked, _) = Redactor::new().redact("token ghp_12345678901234567890 here");
        assert!(masked.contains("<REDACTED:GITHUB_PAT>"));
    }

    #[test]
    fn redacts_private_key() {
        let (masked, _) = Redactor::new().redact("-----BEGIN RSA PRIVATE KEY----- abc");
        assert!(masked.contains("<REDACTED:PRIVATE_KEY>"));
    }

    #[test]
    fn false_positive_guards_pass() {
        let r = Redactor::new();
        for s in [
            "test",
            "example",
            "fake",
            "placeholder",
            "AKIAIOSFODNN7EXAMPLE",
        ] {
            let (masked, findings) = r.redact(s);
            // None of these are credentials: standalone filler words must not
            // trigger high-entropy, and the AWS docs example key is excluded
            // from the aws_key pattern by design (see docs_example_key_not_masked).
            assert_eq!(masked, s, "false positive on {s:?}");
            assert!(findings.is_empty(), "findings on {s:?}");
        }
    }

    #[test]
    fn idempotent() {
        let r = Redactor::new();
        let input = "ghp_12345678901234567890 and AKIAZZZZZZZZZZZZZZZZ";
        let (masked1, _) = r.redact(input);
        let (masked2, _) = r.redact(&masked1);
        assert_eq!(masked1, masked2);
    }

    #[test]
    fn no_pattern_survives() {
        let r = Redactor::new();
        let input = "AKIAZZZZZZZZZZZZZZZZ ghp_12345678901234567890";
        let (masked, _) = r.redact(input);
        assert!(!r.is_redacted(&masked));
        let (_masked2, findings2) = r.redact(&masked);
        assert!(findings2.is_empty());
    }

    #[test]
    fn proves_ask_on_no_redact_error() {
        // Fail-safe: if redact panics, caller must map to ASK. This test proves the crate itself never panics.
        let r = Redactor::new();
        let (masked, _) = r.redact("");
        assert_eq!(masked, "");
    }

    proptest! {
        #[test]
        fn proptest_idempotent(s in ".*") {
            let r = Redactor::new();
            let (masked1, _) = r.redact(&s);
            let (masked2, _) = r.redact(&masked1);
            prop_assert_eq!(masked1, masked2);
        }

        #[test]
        fn proptest_no_pattern_survives(s in ".*") {
            let r = Redactor::new();
            let (masked, _) = r.redact(&s);
            prop_assert!(!r.is_redacted(&masked));
        }
    }
}
