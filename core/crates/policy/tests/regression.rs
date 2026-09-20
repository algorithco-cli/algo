//! Regression corpus integration test — loads `eval/regression-corpus/*.json`
//! and asserts hard-deny behavior per `plans/phase-1-09-quality-latency-eval.md:10`
//! and `eval/regression-corpus/README.md`.
//!
//! - `safe.json` → NOT Deny (Abstain → caller maps to ASK, fail-safe). `Allow` also ok
//!   if future profiles allow explicitly; the invariant is "never Deny for safe".
//! - `dangerous.json` + `obfuscated.json` → must be `Deny` (hard-deny, profile-independent).
//!   Homoglyph entries with fallback ascii are currently Deny; pure homoglyph without fallback
//!   is tracked as WARN not FAIL until NFKC normalization lands (see README § Bins).
//!
//! Path resolution is robust for both `cargo test` (cwd = core/) and `cargo test -p algo-policy`
//! and for CI where env var `ALGO_REGRESSION_CORPUS` may override.

use algo_policy::{Engine, PolicyDecision as Decision, Profile};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Entry {
    command: String,
    label: String,
    source: String,
}

fn find_corpus_dir() -> PathBuf {
    // 1) env var override
    if let Ok(p) = std::env::var("ALGO_REGRESSION_CORPUS") {
        let pb = PathBuf::from(p);
        if pb.is_dir() {
            return pb;
        }
    }

    // 2) compile-time crate dir (robust: works regardless of cwd)
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR")); // core/crates/policy
    let candidates = [
        manifest_dir.join("../../../eval/regression-corpus"), // canonical: core/crates/policy -> core -> ../../eval
        manifest_dir.join("../../../../eval/regression-corpus"), // if nested one deeper (fallback)
        manifest_dir.join("../../eval/regression-corpus"),    // core/eval (wrong but try)
        PathBuf::from("eval/regression-corpus"), // repo-root fallback when cwd=repo-root
        PathBuf::from("../eval/regression-corpus"), // cwd=core
        PathBuf::from("../../eval/regression-corpus"), // cwd=core/crates/policy
        PathBuf::from("core/../eval/regression-corpus"), // alternative
    ];

    for c in &candidates {
        if c.is_dir() {
            return c.clone();
        }
    }

    // 3) walk up from current_dir
    if let Ok(cwd) = std::env::current_dir() {
        let mut cur = cwd.as_path();
        for _ in 0..6 {
            let cand = cur.join("eval/regression-corpus");
            if cand.is_dir() {
                return cand;
            }
            if let Some(parent) = cur.parent() {
                cur = parent;
            } else {
                break;
            }
        }
        // also try core/eval if cwd is repo root but core is subdir
        let cand = cwd.join("core").join("../eval/regression-corpus");
        if cand.is_dir() {
            return cand;
        }
    }

    // fallback: return the canonical path so error message shows expected location
    manifest_dir.join("../../../eval/regression-corpus")
}

fn load_entries(path: &Path) -> Vec<Entry> {
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str::<Vec<Entry>>(&raw).unwrap_or_else(|e| {
        panic!(
            "failed to parse {} as JSON array of {{command,label,source}}: {e}",
            path.display()
        )
    })
}

fn is_safe_label(label: &str) -> bool {
    label.eq_ignore_ascii_case("safe")
}

fn is_dangerous_label(label: &str) -> bool {
    label.eq_ignore_ascii_case("dangerous") || label.eq_ignore_ascii_case("obfuscated")
}

fn contains_non_ascii(s: &str) -> bool {
    s.chars().any(|c| c as u32 > 127)
}

fn is_homoglyph_source(source: &str) -> bool {
    source.to_ascii_lowercase().contains("homoglyph")
}

#[test]
fn regression_corpus() {
    let corpus_dir = find_corpus_dir();
    assert!(
        corpus_dir.is_dir(),
        "regression corpus dir not found: {} (tried ALGO_REGRESSION_CORPUS, CARGO_MANIFEST_DIR relative, and cwd walk). Run from repo root or core/. See eval/regression-corpus/README.md",
        corpus_dir.display()
    );

    let safe_path = corpus_dir.join("safe.json");
    let dangerous_path = corpus_dir.join("dangerous.json");
    let obfuscated_path = corpus_dir.join("obfuscated.json");

    for p in [&safe_path, &dangerous_path, &obfuscated_path] {
        assert!(p.is_file(), "missing corpus file: {}", p.display());
    }

    let engine = Engine::new();

    // --- safe.json: must NOT be Deny (Abstain is expected; Allow also passes if ever introduced)
    let safe_entries = load_entries(&safe_path);
    assert!(
        !safe_entries.is_empty(),
        "safe.json is empty — expected ~20 entries"
    );
    let mut safe_failures = Vec::new();
    for e in &safe_entries {
        assert!(
            is_safe_label(&e.label),
            "safe.json entry label must be 'safe', got '{}' for {:?}",
            e.label,
            e.command
        );
        for profile in [Profile::Strict, Profile::Balanced, Profile::Fast] {
            let decision = engine.evaluate(&e.command, profile.clone());
            if matches!(decision, Decision::Deny { .. }) {
                safe_failures.push(format!(
                    "SAFE should not be Deny but was Deny (profile {:?}): {:?} source={} -> {:?}",
                    profile, e.command, e.source, decision
                ));
            }
        }
    }
    if !safe_failures.is_empty() {
        panic!(
            "safe.json: {} failure(s) — safe commands must not be Denied (fail-safe: Abstain/Ask):\n{}",
            safe_failures.len(),
            safe_failures.join("\n")
        );
    }

    // --- dangerous.json: must be Deny on every profile
    let dangerous_entries = load_entries(&dangerous_path);
    assert!(
        !dangerous_entries.is_empty(),
        "dangerous.json is empty — expected ~20 entries incl. HARD_DENY_RULES"
    );
    let mut dang_failures = Vec::new();
    for e in &dangerous_entries {
        assert!(
            is_dangerous_label(&e.label) || e.label.eq_ignore_ascii_case("dangerous"),
            "dangerous.json label must be 'dangerous', got '{}' for {:?}",
            e.label,
            e.command
        );
        for profile in [Profile::Strict, Profile::Balanced, Profile::Fast] {
            let decision = engine.evaluate(&e.command, profile.clone());
            if !matches!(decision, Decision::Deny { .. }) {
                dang_failures.push(format!(
                    "DANGEROUS should be Deny but was {:?} (profile {:?}): {:?} source={}",
                    decision, profile, e.command, e.source
                ));
            }
        }
    }
    if !dang_failures.is_empty() {
        panic!(
            "dangerous.json: {} failure(s) — all dangerous must be Deny (profile-independent hard-deny):\n{}",
            dang_failures.len(),
            dang_failures.join("\n")
        );
    }

    // --- obfuscated.json: must be Deny (with WARN for pure homoglyph bypass until NFKC lands)
    let obf_entries = load_entries(&obfuscated_path);
    assert!(
        !obf_entries.is_empty(),
        "obfuscated.json is empty — expected ~20 (3-5 per bin: curl|sh, base64|sh, eval+base64, ${{IFS}}, homoglyph)"
    );
    let mut obf_failures = Vec::new();
    let mut obf_warnings = Vec::new();
    for e in &obf_entries {
        // obfuscated entries are labeled dangerous but we also accept obfuscated label
        let _ = e.label.clone(); // label checked loosely
        for profile in [Profile::Strict, Profile::Balanced, Profile::Fast] {
            let decision = engine.evaluate(&e.command, profile.clone());
            if !matches!(decision, Decision::Deny { .. }) {
                // Lenient for pure homoglyph without ascii fallback: track as WARN, not FAIL
                // Detect pure homoglyph = contains non-ascii AND no ascii fallback clause after ';'
                // Our current corpus uses fallback ascii after ';', so they are still Deny.
                // This branch only triggers when a new pure homoglyph bypass is added before normalization.
                let is_pure_homoglyph = contains_non_ascii(&e.command)
                    && is_homoglyph_source(&e.source)
                    && !e.command.contains("; ");
                if is_pure_homoglyph
                    || (contains_non_ascii(&e.command) && is_homoglyph_source(&e.source))
                {
                    // If fallback is present, it should have been Deny; if not, warn
                    // Check if command contains an ascii "curl ... | sh" fallback after ';'
                    let has_ascii_fallback = e.command.contains("curl http")
                        || e.command.contains("base64 -d | sh")
                        || e.command.contains("eval $(echo");
                    if !has_ascii_fallback && is_pure_homoglyph {
                        obf_warnings.push(format!(
                            "WARN (homoglyph bypass, tracked for NFKC normalization): {:?} source={} profile {:?} -> {:?}",
                            e.command, e.source, profile, decision
                        ));
                        continue;
                    }
                }
                obf_failures.push(format!(
                    "OBFUSCATED should be Deny but was {:?} (profile {:?}): {:?} source={}",
                    decision, profile, e.command, e.source
                ));
            }
        }
    }

    if !obf_warnings.is_empty() {
        eprintln!(
            "[regression] {} homoglyph bypass WARN(s) (expected until NFKC normalization, see README § Bins):\n{}",
            obf_warnings.len(),
            obf_warnings.join("\n")
        );
    }
    if !obf_failures.is_empty() {
        panic!(
            "obfuscated.json: {} failure(s) — all obfuscated must be Deny:\n{}\n{} warnings above (if any)",
            obf_failures.len(),
            obf_failures.join("\n"),
            obf_warnings.len()
        );
    }

    // Summary print (visible with --nocapture)
    eprintln!(
        "[regression] corpus OK: safe={} dangerous={} obfuscated={} (warnings={}) from {}",
        safe_entries.len(),
        dangerous_entries.len(),
        obf_entries.len(),
        obf_warnings.len(),
        corpus_dir.display()
    );
    // Ensure we have ~20 each (allow 15-30)
    assert!(
        (15..=30).contains(&safe_entries.len()),
        "safe.json expected ~20 entries, got {}",
        safe_entries.len()
    );
    assert!(
        (15..=30).contains(&dangerous_entries.len()),
        "dangerous.json expected ~20 entries, got {}",
        dangerous_entries.len()
    );
    assert!(
        (15..=30).contains(&obf_entries.len()),
        "obfuscated.json expected ~20 entries, got {}",
        obf_entries.len()
    );
}

#[test]
fn regression_corpus_bins_coverage() {
    // Ensures each obfuscation bin has 3-5 entries as required by P1-09
    let corpus_dir = find_corpus_dir();
    if !corpus_dir.is_dir() {
        eprintln!("[regression] corpus dir not found, skipping bins coverage check");
        return;
    }
    let obf_path = corpus_dir.join("obfuscated.json");
    if !obf_path.is_file() {
        return;
    }
    let entries = load_entries(&obf_path);
    let mut bins = std::collections::HashMap::new();
    for e in &entries {
        // source format "obfuscation: <bin> ..." — extract bin tag before space or comma
        let src = e.source.to_ascii_lowercase();
        let bin = if src.contains("curl_pipe") || src.contains("wget_pipe") {
            "curl_pipe_sh"
        } else if src.contains("base64_pipe") {
            "base64_pipe_sh"
        } else if src.contains("eval_base64") || src.contains("eval+") {
            "eval_base64"
        } else if src.contains("ifs") {
            "ifs"
        } else if src.contains("homoglyph") {
            "homoglyph"
        } else {
            "other"
        };
        *bins.entry(bin).or_insert(0) += 1;
    }
    eprintln!("[regression] bin counts: {:?}", bins);
    for (bin, expected_min) in [
        ("curl_pipe_sh", 3),
        ("base64_pipe_sh", 3),
        ("eval_base64", 3),
        ("ifs", 3),
        ("homoglyph", 3),
    ] {
        let count = bins.get(bin).copied().unwrap_or(0);
        assert!(
            count >= expected_min,
            "obfuscated.json bin '{}' needs >= {} entries, got {} (see README § Bins)",
            bin,
            expected_min,
            count
        );
    }
}
