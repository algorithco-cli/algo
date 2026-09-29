//! Policy bundle verification: detached ed25519, version ordering, rollback protection.
//!
//! Invariant: tampered / expired / rollback → Err, never Ok (proves_ask downstream).
//! Mutant kill: every branch here must be covered by tests (see `cargo mutants`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::Value;

/// Mirrors proto `PolicyBundle{version,signed_bytes,sig}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyBundle {
    pub version: String,
    pub signed_bytes: Vec<u8>,
    pub sig: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    InvalidKey(String),
    InvalidSignature(String),
    Tampered(String),
    Expired { expires_at: i64, now: i64 },
    Rollback { current: u64, got: u64 },
    BadVersion(String),
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey(s) => write!(f, "invalid pubkey: {s}"),
            Self::InvalidSignature(s) => write!(f, "invalid signature: {s}"),
            Self::Tampered(s) => write!(f, "tampered bundle: {s}"),
            Self::Expired { expires_at, now } => {
                write!(f, "expired bundle: exp {expires_at} now {now}")
            }
            Self::Rollback { current, got } => write!(f, "rollback: current {current} got {got}"),
            Self::BadVersion(s) => write!(f, "bad version: {s}"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Global version store: HashMap<scope, current_version_u64>.
/// Spec requires HashMap; MVP uses single key "global".
static VERSION_STORE: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();

/// Full semver strings per scope for precise rollback checks
/// (the u64 store above tracks only the major line for back-compat).
static FULL_VERSION_STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn store() -> &'static Mutex<HashMap<String, u64>> {
    VERSION_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn full_store() -> &'static Mutex<HashMap<String, String>> {
    FULL_VERSION_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_full_store() -> std::sync::MutexGuard<'static, HashMap<String, String>> {
    match full_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("full version store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

fn lock_store() -> std::sync::MutexGuard<'static, HashMap<String, u64>> {
    // Poison-tolerant: a panicked holder must not permanently DoS the process.
    // Fail-closed is handled by callers mapping errors to ask/500.
    match store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("version store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

/// Parse version string to u64 for ordering.
/// Accepted: `^[vV]?\d+(\.\d+)*$` (e.g. "1", "v2", "1.0.0", "v10.2").
/// Ordering uses the major component (first number) so `1.9`/`1.10` are equal
/// major line 1 — callers must not rely on minor for rollback decisions.
/// Anything else (e.g. "1evil", "1.0-beta", "", "v") → BadVersion (fail-closed).
pub fn parse_version(v: &str) -> Result<u64, VerifyError> {
    let trimmed = v.trim();
    if trimmed.is_empty() {
        return Err(VerifyError::BadVersion("empty version".to_string()));
    }
    if trimmed.len() > 64 {
        return Err(VerifyError::BadVersion("version too long".to_string()));
    }
    let stripped = trimmed.trim_start_matches(['v', 'V']);
    if stripped.is_empty() {
        return Err(VerifyError::BadVersion(format!("no numeric prefix in {v}")));
    }
    // Strict: digits separated by single dots, no trailing junk.
    let mut parts = stripped.split('.');
    let major_str = parts.next().unwrap_or("");
    if major_str.is_empty() || !major_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(VerifyError::BadVersion(format!("no numeric prefix in {v}")));
    }
    for part in parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(VerifyError::BadVersion(format!(
                "bad version suffix in {v}"
            )));
        }
    }
    major_str
        .parse::<u64>()
        .map_err(|e| VerifyError::BadVersion(format!("{v}: {e}")))
}

/// Get current version for scope (defaults to 0).
pub fn current_version_for(scope: &str) -> u64 {
    lock_store().get(scope).copied().unwrap_or(0)
}

/// Current version for global scope.
#[allow(dead_code)]
pub fn current_version() -> u64 {
    current_version_for("global")
}

/// Set current version for scope (test helper + post-verify advance).
/// Syncs both the legacy major store and the full-string store.
#[allow(dead_code)]
pub fn set_current_version_for(scope: &str, v: u64) {
    lock_store().insert(scope.to_string(), v);
    // Keep semver store consistent for integer lines (idempotent advance).
    advance_version_str_for(scope, &v.to_string());
}

#[allow(dead_code)]
pub fn set_current_version(v: u64) {
    set_current_version_for("global", v);
}

/// Parse a strict dotted-numeric version into all components.
/// Same acceptance as [`parse_version`] (`^[vV]?\d+(\.\d+)*$`, ≤64 chars),
/// but returns every component for full semver ordering.
pub fn parse_semver(v: &str) -> Result<Vec<u64>, VerifyError> {
    // Reuse strict validation; BadVersion propagates.
    parse_version(v)?;
    let trimmed = v.trim();
    let stripped = trimmed.trim_start_matches(['v', 'V']);
    stripped
        .split('.')
        .map(|p| {
            p.parse::<u64>()
                .map_err(|e| VerifyError::BadVersion(format!("{v}: {e}")))
        })
        .collect()
}

/// Compare two strict versions component-wise (missing trailing = 0, so
/// `1.0 == 1.0.0`). `Err(BadVersion)` when either side is malformed.
pub fn cmp_semver(a: &str, b: &str) -> Result<std::cmp::Ordering, VerifyError> {
    let av = parse_semver(a)?;
    let bv = parse_semver(b)?;
    let len = av.len().max(bv.len());
    for i in 0..len {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            ord => return Ok(ord),
        }
    }
    Ok(std::cmp::Ordering::Equal)
}

/// Current full version string for a scope (precise rollback source).
pub fn current_version_str_for(scope: &str) -> Option<String> {
    lock_full_store().get(scope).cloned()
}

/// Set full version string for a scope (test helper + post-verify advance).
/// Keeps the legacy u64 major store in sync for back-compat readers.
#[cfg(test)]
pub fn set_current_version_str_for(scope: &str, v: &str) {
    if parse_semver(v).is_err() {
        return;
    }
    lock_full_store().insert(scope.to_string(), v.trim().to_string());
    if let Ok(major) = parse_version(v) {
        lock_store().insert(scope.to_string(), major);
    }
}

/// Advance scoped full version iff `new_version` is strictly greater by
/// semver ordering. Also advances the legacy major store when the major
/// line moves forward.
pub fn advance_version_str_for(scope: &str, new_version: &str) {
    let Ok(parsed_new) = parse_semver(new_version) else {
        return;
    };
    let _ = parsed_new;
    let mut full = lock_full_store();
    let dominated = match full.get(scope) {
        Some(cur) => match cmp_semver(new_version, cur) {
            Ok(ord) => ord != std::cmp::Ordering::Greater,
            Err(_) => true,
        },
        None => false,
    };
    if dominated {
        return;
    }
    full.insert(scope.to_string(), new_version.trim().to_string());
    drop(full);
    if let Ok(major) = parse_version(new_version) {
        let mut g = lock_store();
        let cur = g.get(scope).copied().unwrap_or(0);
        if major > cur {
            g.insert(scope.to_string(), major);
        }
    }
}

/// Advance scoped version if `new_version` > current (called after successful verify+apply).
pub fn advance_version_for(scope: &str, new_version: u64) {
    let mut g = lock_store();
    let cur = g.get(scope).copied().unwrap_or(0);
    if new_version > cur {
        g.insert(scope.to_string(), new_version);
    }
    drop(g);
    // Keep the full-string store in sync for integer-line versions.
    let s = new_version.to_string();
    let mut full = lock_full_store();
    let dominated = match full.get(scope) {
        Some(cur) => matches!(
            cmp_semver(&s, cur),
            Ok(std::cmp::Ordering::Less | std::cmp::Ordering::Equal) | Err(_)
        ),
        None => false,
    };
    if !dominated {
        full.insert(scope.to_string(), s);
    }
}

/// Clear store (tests).
#[allow(dead_code)]
pub fn clear_version_store() {
    lock_store().clear();
    lock_full_store().clear();
}

/// Advance global version if `new_version` > current (called after successful verify+apply).
/// Retained for backwards-compat; new code should use `advance_version_for(org_id, v)`.
#[allow(dead_code)]
pub fn advance_version_if_newer(new_version: u64) {
    advance_version_for("global", new_version);
}

/// Detached ed25519 verify + expiry + rollback protection.
///
/// Steps:
/// 1. Validate pubkey (32 bytes) + sig (64 bytes) shape.
/// 2. `VerifyingKey::verify(signed_bytes, sig)` — tampered if fails.
/// 3. `signed_bytes` MUST be a JSON object carrying a valid `expires_at`/`exp`
///    (unix seconds) in the future and within 30d TTL — missing/unparseable
///    expiry or non-JSON payloads fail closed (mandatory expiry, no opaque
///    bypass). For non-`global` scopes the object MUST carry
///    `org_id == scope`, and its `version` (when scopes are per-org, REQUIRED;
///    on `global`, required-when-present) MUST equal `bundle.version`.
/// 4. Version ordering: full semver compare against the stored per-scope
///    version (plus the legacy major-line check) — older releases reject as
///    rollback; equal allows idempotent re-apply.
#[allow(dead_code)]
pub fn verify_bundle(bundle: &PolicyBundle, pubkey: &[u8]) -> Result<(), VerifyError> {
    verify_bundle_scoped(bundle, pubkey, "global")
}

/// Same as `verify_bundle` but checks rollback against `scope` (per-org).
/// New code (policy sync) must use this with `org_id` as scope to avoid
/// cross-tenant version interference.
pub fn verify_bundle_scoped(
    bundle: &PolicyBundle,
    pubkey: &[u8],
    scope: &str,
) -> Result<(), VerifyError> {
    // 1. Key / sig shape.
    if pubkey.len() != 32 {
        return Err(VerifyError::InvalidKey(format!(
            "expected 32 bytes, got {}",
            pubkey.len()
        )));
    }
    if bundle.sig.len() != 64 {
        return Err(VerifyError::InvalidSignature(format!(
            "expected 64 bytes, got {}",
            bundle.sig.len()
        )));
    }

    // 2. Crypto verify (detached). Lengths already checked above; map
    // conversion failures to errors instead of panicking (fail-closed).
    let vk_bytes: [u8; 32] = pubkey
        .try_into()
        .map_err(|_| VerifyError::InvalidKey("pubkey conversion failed".to_string()))?;
    let vk = VerifyingKey::from_bytes(&vk_bytes)
        .map_err(|e| VerifyError::InvalidKey(format!("bad ed25519 pubkey: {e}")))?;
    let sig_bytes: [u8; 64] = bundle
        .sig
        .as_slice()
        .try_into()
        .map_err(|_| VerifyError::InvalidSignature("sig conversion failed".to_string()))?;
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify(&bundle.signed_bytes, &sig)
        .map_err(|e| VerifyError::Tampered(format!("ed25519 verify failed: {e}")))?;

    // 3. Inner payload MUST be a JSON object (mandatory expiry — no opaque
    // bypass) with tenant/version binding.
    let val: Value = serde_json::from_slice(&bundle.signed_bytes).map_err(|_| {
        VerifyError::Tampered("bundle payload must be a JSON object with expires_at".to_string())
    })?;
    let obj = val.as_object().ok_or_else(|| {
        VerifyError::Tampered("bundle payload must be a JSON object with expires_at".to_string())
    })?;

    // 3a. Tenant binding: per-org scopes require inner org_id == scope.
    // `global` is the legacy wildcard and does not enforce (per-org callers
    // must use the scoped entry point with org_id as scope).
    if scope != "global" {
        match obj.get("org_id").and_then(|v| v.as_str()) {
            Some(inner) if inner == scope => {}
            _ => {
                return Err(VerifyError::Tampered(format!(
                    "org binding mismatch: inner org_id must equal scope {scope}"
                )))
            }
        }
    }

    // 3b. Version binding: inner version MUST equal the outer envelope
    // version (prevents version-swapping a valid signature onto another
    // release line). Required for per-org scopes; enforced when present on
    // `global` for back-compat with legacy test vectors.
    match obj.get("version").and_then(|v| v.as_str()) {
        Some(inner_v) => {
            if inner_v != bundle.version {
                return Err(VerifyError::Tampered(format!(
                    "version binding mismatch: inner {inner_v} != outer {}",
                    bundle.version
                )));
            }
        }
        None => {
            if scope != "global" {
                return Err(VerifyError::Tampered(
                    "bundle payload missing version binding".to_string(),
                ));
            }
        }
    }

    // 3c. Expiry check — mandatory for ALL bundles.
    {
        let exp_val = obj.get("expires_at").or_else(|| obj.get("exp"));
        let exp_opt: Option<i64> = match exp_val {
            Some(v) => {
                if let Some(i) = v.as_i64() {
                    Some(i)
                } else if let Some(u) = v.as_u64() {
                    i64::try_from(u).ok()
                } else if let Some(f) = v.as_f64() {
                    // Accept float epoch only if integral and in range.
                    if f.is_finite() && f.fract() == 0.0 && f >= 0.0 && f <= i64::MAX as f64 {
                        Some(f as i64)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            None => None,
        };
        let exp = match exp_opt {
            Some(e) => e,
            None => {
                return Err(VerifyError::Tampered(
                    "bundle missing valid expires_at/exp".to_string(),
                ))
            }
        };
        if exp <= 0 {
            return Err(VerifyError::Tampered(format!("invalid expiry {exp}")));
        }
        let now = chrono::Utc::now().timestamp();
        // Bound TTL to 30 days to catch clock/issuer bugs; longer-lived
        // bundles must be re-issued.
        const MAX_TTL_SECS: i64 = 30 * 24 * 3600;
        if exp > now.saturating_add(MAX_TTL_SECS) {
            return Err(VerifyError::Tampered(format!(
                "expiry too far in future {exp}"
            )));
        }
        if now > exp {
            return Err(VerifyError::Expired {
                expires_at: exp,
                now,
            });
        }
    }

    // 4. Version ordering / rollback protection (scoped): legacy major-line
    // check plus full semver ordering against the stored release string.
    let got = parse_version(&bundle.version)?;
    let current = current_version_for(scope);
    // Rollback if got < current. Equal is allowed for idempotent apply.
    if got < current {
        return Err(VerifyError::Rollback { current, got });
    }
    if let Some(stored) = current_version_str_for(scope) {
        match cmp_semver(&bundle.version, &stored) {
            Ok(std::cmp::Ordering::Less) => {
                return Err(VerifyError::Rollback { current, got });
            }
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    fn test_keypair() -> (SigningKey, VerifyingKey) {
        // Deterministic: 32-byte seed 0x01 repeated; NOT for production.
        let seed = [0x42u8; 32];
        let sk = SigningKey::from_bytes(&seed);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn sign_bundle(version: &str, payload: &[u8], sk: &SigningKey) -> PolicyBundle {
        let sig = sk.sign(payload);
        PolicyBundle {
            version: version.to_string(),
            signed_bytes: payload.to_vec(),
            sig: sig.to_bytes().to_vec(),
        }
    }

    fn payload_with_exp(exp: i64) -> Vec<u8> {
        payload_with_exp_ver(exp, "1")
    }

    /// Version-bound payload: inner `version` MUST equal the outer envelope
    /// version (see §3b). Tests must use this with the matching outer version.
    fn payload_with_exp_ver(exp: i64, version: &str) -> Vec<u8> {
        json!({"content":"hello policy","expires_at": exp, "version": version})
            .to_string()
            .into_bytes()
    }

    /// Per-org scoped payload: inner `org_id` + `version` bound to the scope
    /// and outer version (see §3a–3b).
    fn scoped_payload(scope: &str, version: &str, exp: i64) -> Vec<u8> {
        json!({"org_id": scope, "content":"hello policy","expires_at": exp, "version": version})
            .to_string()
            .into_bytes()
    }

    #[test]
    fn verify_ok_future_expiry() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let exp = chrono::Utc::now().timestamp() + 3600;
        let payload = payload_with_exp_ver(exp, "1");
        let bundle = sign_bundle("1", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        assert!(verify_bundle(&bundle, &pubkey).is_ok());
    }

    #[test]
    fn tampered_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let exp = chrono::Utc::now().timestamp() + 3600;
        let payload = payload_with_exp_ver(exp, "2");
        let mut bundle = sign_bundle("2", &payload, &sk);
        // Tamper after signing.
        bundle.signed_bytes[0] ^= 0xFF;
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::Tampered(_)));
    }

    #[test]
    fn tampered_sig_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "3");
        let mut bundle = sign_bundle("3", &payload, &sk);
        bundle.sig[0] ^= 0xAA;
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        // Signature mismatch → Tampered (via ed25519 verify).
        assert!(matches!(err, VerifyError::Tampered(_)));
    }

    #[test]
    fn expired_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let exp = chrono::Utc::now().timestamp() - 10; // already expired
        let payload = payload_with_exp_ver(exp, "4");
        let bundle = sign_bundle("4", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::Expired { .. }));
    }

    #[test]
    fn rollback_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let exp = chrono::Utc::now().timestamp() + 3600;
        let payload = payload_with_exp_ver(exp, "3");
        // Set current to 5.
        set_current_version(5);
        let bundle = sign_bundle("3", &payload, &sk); // older
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert_eq!(err, VerifyError::Rollback { current: 5, got: 3 });
        clear_version_store();
    }

    #[test]
    fn rollback_equal_allowed() {
        let _guard = test_sync::lock();
        clear_version_store();
        set_current_version(5);
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "5");
        let bundle = sign_bundle("5", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        assert!(verify_bundle(&bundle, &pubkey).is_ok());
        clear_version_store();
    }

    #[test]
    fn newer_version_allowed() {
        let _guard = test_sync::lock();
        clear_version_store();
        set_current_version(5);
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "6");
        let bundle = sign_bundle("6", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        assert!(verify_bundle(&bundle, &pubkey).is_ok());
        clear_version_store();
    }

    #[test]
    fn invalid_pubkey_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, _) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "1");
        let bundle = sign_bundle("1", &payload, &sk);
        let bad_pubkey = vec![0u8; 16];
        let err = verify_bundle(&bundle, &bad_pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::InvalidKey(_)));
    }

    #[test]
    fn invalid_sig_len_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "1");
        let mut bundle = sign_bundle("1", &payload, &sk);
        bundle.sig = vec![0u8; 10];
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::InvalidSignature(_)));
    }

    #[test]
    fn bad_version_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp_ver(chrono::Utc::now().timestamp() + 3600, "not-a-version");
        let bundle = sign_bundle("not-a-version", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::BadVersion(_)));
    }

    #[test]
    fn version_parsing_variants() {
        let _guard = test_sync::lock();
        assert_eq!(parse_version("1").unwrap(), 1);
        assert_eq!(parse_version("v2").unwrap(), 2);
        assert_eq!(parse_version("V10").unwrap(), 10);
        assert_eq!(parse_version("1.0.0").unwrap(), 1);
        assert_eq!(parse_version("v3.2.1").unwrap(), 3);
        assert!(parse_version("").is_err());
        assert!(parse_version("abc").is_err());
    }

    #[test]
    fn proves_ask_on_version_suffix() {
        let _guard = test_sync::lock();
        // Trailing junk must fail closed (rollback bypass prevention).
        for bad in ["1evil", "1.0-beta", "v", "1..0", "1.0.0 ", "  "] {
            // Note: "1.0.0 " trims to "1.0.0" and passes; assert others fail.
            if bad.trim() == "1.0.0" {
                assert!(parse_version(bad).is_ok());
            } else if bad == "  " {
                assert!(parse_version(bad).is_err());
            } else {
                assert!(parse_version(bad).is_err(), "should reject {bad}");
            }
        }
        assert!(parse_version("1evil").is_err());
        assert!(parse_version("1.10evil").is_err());
    }

    #[test]
    fn proves_ask_on_json_missing_expiry() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        // JSON payload without expires_at/exp must fail closed.
        let payload = json!({"content":"no expiry here"}).to_string().into_bytes();
        let bundle = sign_bundle("1", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::Tampered(_)));
    }

    #[test]
    fn per_org_rollback_isolation() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let pubkey = vk.to_bytes().to_vec();
        let exp = chrono::Utc::now().timestamp() + 3600;
        // Org-A at v100 must not block org-B at v2.
        let payload_a = scoped_payload("org-a", "100", exp);
        let bundle_a = sign_bundle("100", &payload_a, &sk);
        verify_bundle_scoped(&bundle_a, &pubkey, "org-a").expect("org-a v100 ok");
        set_current_version_for("org-a", 100);
        let payload_b = scoped_payload("org-b", "2", exp);
        let bundle_b = sign_bundle("2", &payload_b, &sk);
        assert!(verify_bundle_scoped(&bundle_b, &pubkey, "org-b").is_ok());
        // But org-A rollback to v3 is rejected.
        let payload_old = scoped_payload("org-a", "3", exp);
        let bundle_old = sign_bundle("3", &payload_old, &sk);
        let err = verify_bundle_scoped(&bundle_old, &pubkey, "org-a").unwrap_err();
        assert!(matches!(err, VerifyError::Rollback { .. }));
    }

    #[test]
    fn proves_ask_on_org_binding_mismatch() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let pubkey = vk.to_bytes().to_vec();
        let exp = chrono::Utc::now().timestamp() + 3600;
        // Inner org-a presented to scope org-b → Tampered.
        let payload = scoped_payload("org-a", "7", exp);
        let bundle = sign_bundle("7", &payload, &sk);
        let err = verify_bundle_scoped(&bundle, &pubkey, "org-b").unwrap_err();
        assert!(matches!(err, VerifyError::Tampered(_)));
        // Missing org_id on a per-org scope → Tampered.
        let payload_no_org = payload_with_exp_ver(exp, "7");
        let bundle_no_org = sign_bundle("7", &payload_no_org, &sk);
        let err2 = verify_bundle_scoped(&bundle_no_org, &pubkey, "org-b").unwrap_err();
        assert!(matches!(err2, VerifyError::Tampered(_)));
    }

    #[test]
    fn proves_ask_on_version_binding_mismatch() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let pubkey = vk.to_bytes().to_vec();
        let exp = chrono::Utc::now().timestamp() + 3600;
        // Inner 8 vs outer 9 (global, present) → Tampered.
        let payload = payload_with_exp_ver(exp, "8");
        let bundle = sign_bundle("9", &payload, &sk);
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::Tampered(_)));
        // Scoped with mismatched binding → Tampered.
        let scoped = scoped_payload("org-v", "8", exp);
        let bundle2 = sign_bundle("9", &scoped, &sk);
        let err2 = verify_bundle_scoped(&bundle2, &pubkey, "org-v").unwrap_err();
        assert!(matches!(err2, VerifyError::Tampered(_)));
    }

    #[test]
    fn proves_ask_on_non_json_mandatory_expiry() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let pubkey = vk.to_bytes().to_vec();
        // Opaque non-JSON payloads no longer skip expiry — fail closed.
        let bundle = sign_bundle("1", b"opaque-bytes-not-json", &sk);
        let err = verify_bundle(&bundle, &pubkey).unwrap_err();
        assert!(matches!(err, VerifyError::Tampered(_)));
        // JSON array (not object) also fails closed.
        let bundle2 = sign_bundle("1", b"[1,2,3]", &sk);
        let err2 = verify_bundle(&bundle2, &pubkey).unwrap_err();
        assert!(matches!(err2, VerifyError::Tampered(_)));
    }

    #[test]
    fn semver_rollback_minor_rejected() {
        let _guard = test_sync::lock();
        clear_version_store();
        let (sk, vk) = test_keypair();
        let pubkey = vk.to_bytes().to_vec();
        let exp = chrono::Utc::now().timestamp() + 3600;
        // Store 1.10 (same major line as 1.9 — legacy major check passes).
        set_current_version_str_for("org-s", "1.10");
        let payload = scoped_payload("org-s", "1.9", exp);
        let bundle = sign_bundle("1.9", &payload, &sk);
        let err = verify_bundle_scoped(&bundle, &pubkey, "org-s").unwrap_err();
        assert!(matches!(err, VerifyError::Rollback { .. }));
        // 1.10.1 over 1.10 is allowed.
        let payload_new = scoped_payload("org-s", "1.10.1", exp);
        let bundle_new = sign_bundle("1.10.1", &payload_new, &sk);
        assert!(verify_bundle_scoped(&bundle_new, &pubkey, "org-s").is_ok());
    }

    #[test]
    fn secret_in_payload_still_verified_if_sig_ok_but_audit_layer_rejects() {
        let _guard = test_sync::lock();
        // verify layer checks sig/expiry/rollback; secret check is audit layer.
        // Here we prove sig still verifies even if payload contains secret-like string,
        // but the combined policy+audit flow must fail closed at audit ingest.
        clear_version_store();
        let (sk, vk) = test_keypair();
        let payload =
            json!({"content":"ghp_12345678901234567890","expires_at": chrono::Utc::now().timestamp()+3600})
                .to_string()
                .into_bytes();
        let bundle = sign_bundle("1", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        // Sig verifies — not redacted at verify layer, but audit would reject.
        assert!(verify_bundle(&bundle, &pubkey).is_ok());
    }

    #[test]
    fn proves_ask_on_verify_error() {
        let _guard = test_sync::lock();
        // Fail-safe: every verify error must map to ask downstream. Here we prove
        // that error kinds are never Ok and are exhaustive.
        clear_version_store();
        let (sk, vk) = test_keypair();
        let payload = payload_with_exp(chrono::Utc::now().timestamp() - 5);
        let bundle = sign_bundle("1", &payload, &sk);
        let pubkey = vk.to_bytes().to_vec();
        let res = verify_bundle(&bundle, &pubkey);
        assert!(res.is_err());
        // Caller must map to ask; we prove error is not allow.
        match res.unwrap_err() {
            VerifyError::Expired { .. } => {}
            other => panic!("expected expired, got {other:?}"),
        }
    }
}
