//! Policy store: publish → sign with test key, verify_and_apply via `verify.rs`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde_json::json;

use crate::verify::{
    advance_version_for, parse_version, verify_bundle_scoped, PolicyBundle, VerifyError,
};

/// Signing key loader.
///
/// Production must set `ALGO_POLICY_SIGNING_SEED_HEX` (64 hex chars = 32 bytes,
/// provisioned via OpenBao / SOPS+age per Phase 3 stack). If unset, we fall back
/// to the deterministic test seed so local MVP/tests keep working, but emit a
/// loud warning — this fallback must never be used in production.
/// Key management is human-review-gated.
fn load_signing_key() -> SigningKey {
    if let Ok(hex) = std::env::var("ALGO_POLICY_SIGNING_SEED_HEX") {
        let hex = hex.trim();
        if hex.len() == 64 {
            let mut seed = [0u8; 32];
            let mut ok = true;
            for i in 0..32 {
                match u8::from_str_radix(&hex[2 * i..2 * i + 2], 16) {
                    Ok(b) => seed[i] = b,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                return SigningKey::from_bytes(&seed);
            }
            tracing::warn!(
                "ALGO_POLICY_SIGNING_SEED_HEX malformed; failing closed is preferred in prod"
            );
        } else {
            tracing::warn!("ALGO_POLICY_SIGNING_SEED_HEX wrong length; expected 64 hex chars");
        }
    }
    tracing::warn!("using deterministic test signing key (NOT for production)");
    // 32-byte deterministic seed — stable across runs for tests.
    let seed = [0x42u8; 32];
    SigningKey::from_bytes(&seed)
}

/// Deterministic test keypair for MVP (NOT production; pinned key in verify).
fn test_signing_key() -> SigningKey {
    load_signing_key()
}

fn test_verifying_key() -> VerifyingKey {
    test_signing_key().verifying_key()
}

/// Return test pubkey bytes for clients to pin.
pub fn test_pubkey_bytes() -> Vec<u8> {
    test_verifying_key().to_bytes().to_vec()
}

/// In-memory policy store (scaffold; Postgres in production).
/// Holds current bundle per org (keyed by org_id or "global").
pub struct PolicyStore {
    inner: Mutex<HashMap<String, StoredPolicy>>,
}

#[derive(Debug, Clone)]
struct StoredPolicy {
    version: u64,
    bundle: PolicyBundle,
}

impl PolicyStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Global singleton for the axum app (MVP).
    #[allow(dead_code)]
    pub fn global() -> &'static Self {
        static GLOBAL: OnceLock<PolicyStore> = OnceLock::new();
        GLOBAL.get_or_init(PolicyStore::new)
    }

    fn lock_inner(&self) -> std::sync::MutexGuard<'_, HashMap<String, StoredPolicy>> {
        match self.inner.lock() {
            Ok(g) => g,
            Err(poisoned) => {
                tracing::warn!("policy store mutex poisoned; recovering inner");
                poisoned.into_inner()
            }
        }
    }

    /// Publish: create new bundle, sign with test key, store, return bundle.
    /// `payload_content` is user-supplied policy DSL (opaque here); we wrap with metadata.
    pub fn publish(&self, org_id: &str, payload_content: &str) -> PolicyBundle {
        let mut guard = self.lock_inner();
        let current = guard.get(org_id).map(|s| s.version).unwrap_or(0);
        let next = current + 1;
        let version_str = next.to_string();
        let expires_at = Utc::now().timestamp() + 3600; // 1h validity
        let signed_json = json!({
            "org_id": org_id,
            "version": version_str,
            "content": payload_content,
            "expires_at": expires_at,
        })
        .to_string();
        let signed_bytes = signed_json.into_bytes();
        let sk = test_signing_key();
        let sig = sk.sign(&signed_bytes).to_bytes().to_vec();
        let bundle = PolicyBundle {
            version: version_str.clone(),
            signed_bytes,
            sig,
        };
        guard.insert(
            org_id.to_string(),
            StoredPolicy {
                version: next,
                bundle: bundle.clone(),
            },
        );
        // Advance per-org verify store for rollback protection.
        // (Global store no longer advanced here to avoid cross-tenant interference.)
        advance_version_for(org_id, next);
        bundle
    }

    /// Direct publish with raw bytes (for tests that craft expiry).
    /// Fail-closed on bad version instead of storing version 0.
    #[allow(dead_code)]
    pub fn publish_raw(
        &self,
        org_id: &str,
        version: &str,
        signed_bytes: Vec<u8>,
    ) -> Result<PolicyBundle, VerifyError> {
        let parsed = parse_version(version)?;
        let sk = test_signing_key();
        let sig = sk.sign(&signed_bytes).to_bytes().to_vec();
        let bundle = PolicyBundle {
            version: version.to_string(),
            signed_bytes,
            sig,
        };
        let mut guard = self.lock_inner();
        guard.insert(
            org_id.to_string(),
            StoredPolicy {
                version: parsed,
                bundle: bundle.clone(),
            },
        );
        advance_version_for(org_id, parsed);
        Ok(bundle)
    }

    /// Verify bundle (detached sig + expiry + rollback) and apply if newer.
    /// Rollback state is per-org; crypto verify happens before taking the lock.
    pub fn verify_and_apply(&self, org_id: &str, bundle: &PolicyBundle) -> Result<(), VerifyError> {
        let pubkey = test_pubkey_bytes();
        // Scoped crypto+expiry+rollback pre-check (against shared version store).
        verify_bundle_scoped(bundle, &pubkey, org_id)?;
        let got = parse_version(&bundle.version)?;
        let mut guard = self.lock_inner();
        let current = guard.get(org_id).map(|s| s.version).unwrap_or(0);
        if got < current {
            return Err(VerifyError::Rollback { current, got });
        }
        // Allow equal (idempotent) and greater.
        if got > current {
            guard.insert(
                org_id.to_string(),
                StoredPolicy {
                    version: got,
                    bundle: bundle.clone(),
                },
            );
            advance_version_for(org_id, got);
        }
        Ok(())
    }

    pub fn get(&self, org_id: &str, version: &str) -> Option<PolicyBundle> {
        let guard = self.lock_inner();
        if version.is_empty() || version == "latest" {
            guard.get(org_id).map(|s| s.bundle.clone())
        } else {
            // Exact version lookup — only if stored version matches.
            guard.get(org_id).and_then(|s| {
                if s.bundle.version == version {
                    Some(s.bundle.clone())
                } else {
                    None
                }
            })
        }
    }

    #[allow(dead_code)]
    pub fn latest_version(&self, org_id: &str) -> Option<String> {
        self.lock_inner()
            .get(org_id)
            .map(|s| s.bundle.version.clone())
    }

    /// Clear store (tests).
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.lock_inner().clear();
    }
}

impl Default for PolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;
    use crate::verify::{clear_version_store, verify_bundle};
    use ed25519_dalek::Signer;

    #[test]
    fn publish_and_verify_apply_ok() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let bundle = store.publish("org-1", "allow *");
        // Verify with pinned key should succeed.
        let pubkey = test_pubkey_bytes();
        assert!(verify_bundle(&bundle, &pubkey).is_ok());
        // Store already has it; re-apply idempotent.
        assert!(store.verify_and_apply("org-1", &bundle).is_ok());
    }

    #[test]
    fn publish_increments_version() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let b1 = store.publish("org-x", "v1 content");
        let b2 = store.publish("org-x", "v2 content");
        assert_ne!(b1.version, b2.version);
        assert_eq!(b1.version, "1");
        assert_eq!(b2.version, "2");
    }

    #[test]
    fn verify_and_apply_rejects_tampered() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let mut bundle = store.publish("org-t", "hello");
        bundle.signed_bytes[0] ^= 0xFF;
        let res = store.verify_and_apply("org-t", &bundle);
        assert!(res.is_err());
        assert!(matches!(
            res.unwrap_err(),
            crate::verify::VerifyError::Tampered(_)
        ));
    }

    #[test]
    fn verify_and_apply_rejects_rollback() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let _b1 = store.publish("org-r", "first");
        let _b2 = store.publish("org-r", "second"); // version 2
                                                    // Craft old bundle version 1 with valid sig for that payload.
        let sk = test_signing_key();
        let payload = json!({"org_id":"org-r","version":"1","content":"old","expires_at": Utc::now().timestamp()+3600})
            .to_string()
            .into_bytes();
        let sig = sk.sign(&payload).to_bytes().to_vec();
        let old_bundle = PolicyBundle {
            version: "1".to_string(),
            signed_bytes: payload,
            sig,
        };
        let res = store.verify_and_apply("org-r", &old_bundle);
        assert!(res.is_err());
        match res.unwrap_err() {
            crate::verify::VerifyError::Rollback { current: 2, got: 1 } => {}
            e => panic!("wrong error {e:?}"),
        }
    }

    #[test]
    fn expired_rejected_via_verify() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let sk = test_signing_key();
        let payload = json!({"org_id":"org-e","version":"99","content":"exp","expires_at": Utc::now().timestamp()-100})
            .to_string()
            .into_bytes();
        let sig = sk.sign(&payload).to_bytes().to_vec();
        let bundle = PolicyBundle {
            version: "99".to_string(),
            signed_bytes: payload,
            sig,
        };
        let res = store.verify_and_apply("org-e", &bundle);
        assert!(matches!(
            res.unwrap_err(),
            crate::verify::VerifyError::Expired { .. }
        ));
    }

    #[test]
    fn get_latest_and_exact() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let b = store.publish("org-g", "content");
        assert_eq!(store.get("org-g", "latest").unwrap().version, b.version);
        assert_eq!(store.get("org-g", &b.version).unwrap().version, b.version);
        assert!(store.get("org-g", "999").is_none());
        assert!(store.get("unknown", "latest").is_none());
    }

    #[test]
    fn publish_raw_and_apply() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let payload = json!({"content":"raw","expires_at": Utc::now().timestamp()+3600})
            .to_string()
            .into_bytes();
        let bundle = store
            .publish_raw("org-raw", "5", payload)
            .expect("publish_raw ok");
        assert_eq!(bundle.version, "5");
        assert!(store.verify_and_apply("org-raw", &bundle).is_ok());
    }

    #[test]
    fn publish_raw_rejects_bad_version_fail_closed() {
        let _guard = test_sync::lock();
        clear_version_store();
        let store = PolicyStore::new();
        let payload = json!({"content":"raw","expires_at": Utc::now().timestamp()+3600})
            .to_string()
            .into_bytes();
        let err = store.publish_raw("org-raw", "1evil", payload).unwrap_err();
        assert!(matches!(err, crate::verify::VerifyError::BadVersion(_)));
    }
}
