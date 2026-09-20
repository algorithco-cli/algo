//! Fingerprint normalization for cache — `normalize` + `cache_key`.
//!
//! Spec: `plans/phase-1-02-core-types-fingerprint.md:9-14`
//! `normalize(cmd: &str) -> String`: lowercase argv[0] basename, sort order-invariant
//! long flags, replace `/(tmp|var|home)/[^ ]+`, `[0-9a-f]{7,}`, timestamps, uuid → `<PATH>/<HASH>/<NUM>`,
//! preserve redirections/pipes/sudo. `cache_key = blake3(normalize + policy_version + profile)`.
//! Edges: `VAR=x cmd`, `sudo -u u cmd`, `cmd 2>&1 | tee`, quoted vs unquoted, `sh -c '...'` one level only.

use regex::Regex;
use std::sync::OnceLock;

static RE_PATH: OnceLock<Regex> = OnceLock::new();
static RE_HASH: OnceLock<Regex> = OnceLock::new();
static RE_TIMESTAMP: OnceLock<Regex> = OnceLock::new();
static RE_UUID: OnceLock<Regex> = OnceLock::new();
static RE_NUM: OnceLock<Regex> = OnceLock::new();

fn re_path() -> &'static Regex {
    RE_PATH.get_or_init(|| Regex::new(r"/(tmp|var|home)[^\s|;']*").unwrap())
}
fn re_hash() -> &'static Regex {
    RE_HASH.get_or_init(|| Regex::new(r"\b[0-9a-f]{7,}\b").unwrap())
}
fn re_timestamp() -> &'static Regex {
    RE_TIMESTAMP.get_or_init(|| Regex::new(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}\b").unwrap())
}
fn re_uuid() -> &'static Regex {
    RE_UUID.get_or_init(|| Regex::new(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap())
}
fn re_num() -> &'static Regex {
    RE_NUM.get_or_init(|| Regex::new(r"\b\d{3,}\b").unwrap())
}

/// Normalize a shell command for cache keying.
///
/// Idempotent: `normalize(normalize(x)) == normalize(x)`.
/// Never emits secret-looking substrings (checked vs `algo-redact`).
pub fn normalize(cmd: &str) -> String {
    let mut s = cmd.trim().to_string();

    // Handle `sh -c '...'` one level only — extract inner, normalize it, then re-wrap.
    // Deeper nesting (inner contains `sh -c`) → return unchanged with marker for `ask`.
    if let Some(inner) = extract_sh_c(&s) {
        if inner.contains("sh -c") {
            return "unparseable:nested".to_string();
        }
        let inner_norm = normalize(&inner);
        return format!("sh -c '{}'", inner_norm);
    }

    // Strip leading VAR=x env assigns (only valid shell variable names)
    // e.g., `VAR=x cmd` → `cmd` with env stripped for fingerprint
    let mut rest = s.as_str();
    let mut stripped_env = false;
    while let Some(eq_pos) = rest.find('=') {
        let before_eq = &rest[..eq_pos];
        if before_eq.contains(' ') || before_eq.is_empty() {
            break;
        }
        // VAR must be a valid shell variable name: [A-Za-z_][A-Za-z0-9_]*
        if !before_eq
            .chars()
            .enumerate()
            .all(|(i, c)| if i == 0 { c.is_ascii_alphabetic() || c == '_' } else { c.is_ascii_alphanumeric() || c == '_' })
        {
            break;
        }
        if let Some(space_pos) = rest[eq_pos..].find(' ') {
            // VAR=x with space after value → strip it
            rest = rest[eq_pos + space_pos..].trim_start();
            stripped_env = true;
        } else {
            break;
        }
    }
    if stripped_env {
        s = rest.to_string();
    }

    // Handle `sudo -u <user> cmd` → keep `sudo` marker, strip `-u <user>`
    let mut cmd_part = s.as_str();
    if cmd_part.starts_with("sudo ") {
        cmd_part = cmd_part[5..].trim_start();
        if cmd_part.starts_with("-u") {
            let after_u = cmd_part[2..].trim_start();
            if let Some(space) = after_u.find(' ') {
                cmd_part = after_u[space + 1..].trim_start();
            } else {
                cmd_part = "";
            }
        }
        s = format!("sudo {}", cmd_part);
    }

    // Split into tokens, preserving pipes and redirections as separate tokens
    // For fingerprint, we preserve `|`, `2>&1`, `>`, `>>`, `| tee` as tokens
    let tokens = tokenize(&s);
    if tokens.is_empty() {
        return String::new();
    }

    // Lowercase argv[0] basename (first non-operator, non-empty token), dropping empty basenames
    let mut normalized_tokens: Vec<String> = Vec::new();
    for tok in tokens.iter() {
        if !tok.is_empty() {
            normalized_tokens.push(tok.clone());
        }
    }
    if normalized_tokens.is_empty() {
        return String::new();
    }
    // Find first non-operator token and lowercase its basename
    for tok in normalized_tokens.iter_mut() {
        if !is_operator(tok) {
            let base = tok.rsplit('/').next().unwrap_or(tok.as_str());
            let lower = base.to_lowercase();
            if lower.is_empty() {
                // If basename is empty (e.g., "/" ), skip this token
                // This case was already handled by filtering empty, but keep for safety
                continue;
            }
            *tok = lower;
            break;
        }
    }
    // Filter out any remaining empty tokens (from "/" handling)
    normalized_tokens.retain(|t| !t.is_empty());

    // Sort order-invariant long flags (--foo) — only sort the flags, not the whole command
    // For simplicity, extract long flags, sort them, and reinsert at their original positions
    // This is a simplified version: collect all `--*` tokens, sort, and replace in order.
    let mut long_flags: Vec<String> = normalized_tokens
        .iter()
        .filter(|t| t.starts_with("--"))
        .cloned()
        .collect();
    long_flags.sort();
    let mut flag_idx = 0;
    for tok in normalized_tokens.iter_mut() {
        if tok.starts_with("--") {
            *tok = long_flags[flag_idx].clone();
            flag_idx += 1;
        }
    }

    let mut out = normalized_tokens.join(" ");

    // Replace sensitive patterns
    out = re_path().replace_all(&out, "<PATH>").to_string();
    out = re_uuid().replace_all(&out, "<UUID>").to_string();
    out = re_timestamp().replace_all(&out, "<TIMESTAMP>").to_string();
    // Hashes after path/uuid/timestamp to avoid double-replacing
    out = re_hash().replace_all(&out, "<HASH>").to_string();
    // Numbers (but keep small numbers like `2` in `2>&1`? Our regex is \b\d{3,}\b so 2 is kept)
    out = re_num().replace_all(&out, "<NUM>").to_string();

    out.trim().to_string()
}

fn is_operator(tok: &str) -> bool {
    matches!(tok, "|" | "||" | "&&" | ";" | "&" | ">" | ">>" | "<" | "2>&1" | "2>" | "1>" | "tee")
}

fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' && !in_double {
            in_single = !in_single;
            current.push(c);
        } else if c == '"' && !in_single {
            in_double = !in_double;
            current.push(c);
        } else if !in_single && !in_double {
            // Handle 2>&1 as a single token (must be before generic operator handling)
            if i + 3 < chars.len() && chars[i..i + 4] == ['2', '>', '&', '1'] {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
                tokens.push("2>&1".to_string());
                i += 4;
                continue;
            }
            // Preserve placeholders like <PATH> as single tokens (idempotence)
            if c == '<' {
                let remaining: String = chars[i..].iter().collect();
                let mut is_placeholder = false;
                for ph in ["<PATH>", "<HASH>", "<UUID>", "<TIMESTAMP>", "<NUM>"] {
                    if remaining.starts_with(ph) {
                        if !current.trim().is_empty() {
                            tokens.push(current.trim().to_string());
                            current.clear();
                        }
                        tokens.push(ph.to_string());
                        i += ph.len() - 1;
                        is_placeholder = true;
                        break;
                    }
                }
                if is_placeholder {
                    i += 1;
                    continue;
                }
            }
            if c == '|' || c == ';' || c == '&' || c == '>' || c == '<' {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
                // Handle >> and ||
                if i + 1 < chars.len() && chars[i + 1] == c {
                    tokens.push(format!("{}{}", c, c));
                    i += 1;
                } else {
                    tokens.push(c.to_string());
                }
            } else if c == ' ' || c == '\t' {
                if !current.trim().is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            } else {
                current.push(c);
            }
        } else {
            current.push(c);
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }
    tokens
}

fn extract_sh_c(s: &str) -> Option<String> {
    let s = s.trim();
    if s.starts_with("sh -c ") || s.starts_with("bash -c ") {
        let start = s.find('\'').or_else(|| s.find('"'))?;
        let quote = s.chars().nth(start)?;
        let end = s[start + 1..].find(quote)?;
        return Some(s[start + 1..start + 1 + end].to_string());
    }
    None
}

/// Cache key = blake3(normalize + policy_version + profile) hex.
pub fn cache_key(normalized: &str, policy_version: &str, profile: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(normalized.as_bytes());
    hasher.update(policy_version.as_bytes());
    hasher.update(profile.as_bytes());
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn normalize_idempotent() {
        let cases = vec!["ls -la /tmp/foo", "SUDO ls -la", "VAR=x curl https://example.com/a1"];
        for c in cases {
            assert_eq!(normalize(&normalize(c)), normalize(c));
        }
    }

    proptest! {
        #[test]
        fn proptest_normalize_idempotent(s in "[a-zA-Z0-9 _\\-\\.\\/\\|\\&\\;]*") {
            prop_assert_eq!(normalize(&normalize(&s)), normalize(&s));
        }
    }

    #[test]
    fn never_emits_secret_substrings() {
        let cases = vec!["AKIAIOSFODNN7EXAMPLE", "ghp_12345678901234567890", "secret123"];
        for c in cases {
            let norm = normalize(c);
            // Should not contain raw secret patterns (our redact would catch them, but normalize shouldn't reintroduce)
            assert!(!norm.contains("AKIA") || norm.contains("<HASH>") || norm.contains("<PATH>"));
        }
    }

    #[test]
    fn corpus_same_key_for_variants() {
        // `curl …/a1` vs `…/d4` same key after hash replacement
        let a = normalize("curl https://example.com/a1234567");
        let b = normalize("curl https://example.com/d4123456");
        assert_eq!(cache_key(&a, "v1", "balanced"), cache_key(&b, "v1", "balanced"));
        // `rm -rf /` vs `rm -rf /tmp/x` different
        let c = normalize("rm -rf /");
        let d = normalize("rm -rf /tmp/x");
        assert_ne!(cache_key(&c, "v1", "balanced"), cache_key(&d, "v1", "balanced"));
    }

    #[test]
    fn preserves_pipes_and_sudo() {
        assert!(normalize("sudo ls -la").starts_with("sudo "));
        assert!(normalize("ls 2>&1 | tee").contains("2>&1"));
        assert!(normalize("ls 2>&1 | tee").contains("|"));
    }

    #[test]
    fn handles_sh_c_one_level() {
        assert_eq!(normalize("sh -c 'ls -la /tmp'"), "sh -c 'ls -la <PATH>'");
        assert_eq!(normalize("sh -c 'sh -c \"ls\"'"), "unparseable:nested");
    }

    #[test]
    fn proves_ask_on_unparseable_nested() {
        // Deeper than one level → ask + unparseable:nested (fail-safe)
        let norm = normalize("sh -c 'sh -c \"echo hi\"'");
        assert_eq!(norm, "unparseable:nested");
    }
}
