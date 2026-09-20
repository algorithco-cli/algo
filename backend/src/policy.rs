//! Policy store: publish → sign with test key, verify_and_apply via `verify.rs`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use serde_json::json;

use crate::verify::{
    advance_version_if_newer, parse_version, verify_bundle, PolicyBundle, VerifyError,
};

/// Deterministic test keypair for MVP (NOT production; pinned key in verify).
fn test_signing_key() -> SigningKey {
    // 32-byte deterministic seed — stable across runs for tests.
    let seed = [0x42u8; 32];
    SigningKey::from_bytes(&seed)
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

    /// Publish: create new bundle, sign with test key, store, return bundle.
    /// `payload_content` is user-supplied policy DSL (opaque here); we wrap with metadata.
    pub fn publish(&self, org_id: &str, payload_content: &str) -> PolicyBundle {
        let mut guard = self.inner.lock().expect("policy store poisoned");
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
        // Advance global verify store for rollback protection too.
        advance_version_if_newer(next);
        bundle
    }

    /// Direct publish with raw bytes (for tests that craft expiry).
    pub fn publish_raw(&self, org_id: &str, version: &str, signed_bytes: Vec<u8>) -> PolicyBundle {
        let sk = test_signing_key();
        let sig = sk.sign(&signed_bytes).to_bytes().to_vec();
        let bundle = PolicyBundle {
            version: version.to_string(),
            signed_bytes,
            sig,
        };
        let parsed = parse_version(version).unwrap_or(0);
        let mut guard = self.inner.lock().expect("policy store poisoned");
        guard.insert(
            org_id.to_string(),
            StoredPolicy {
                version: parsed,
                bundle: bundle.clone(),
            },
        );
        advance_version_if_newer(parsed);
        bundle
    }

    /// Verify bundle (detached sig + expiry + rollback) and apply if newer.
    pub fn verify_and_apply(&self, org_id: &str, bundle: &PolicyBundle) -> Result<(), VerifyError> {
        let pubkey = test_pubkey_bytes();
        verify_bundle(bundle, &pubkey)?;
        let got = parse_version(&bundle.version)?;
        let mut guard = self.inner.lock().expect("policy store poisoned");
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
            advance_version_if_newer(got);
        }
        Ok(())
    }

    pub fn get(&self, org_id: &str, version: &str) -> Option<PolicyBundle> {
        let guard = self.inner.lock().expect("policy store poisoned");
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
        self.inner
            .lock()
            .expect("policy store poisoned")
            .get(org_id)
            .map(|s| s.bundle.version.clone())
    }

    /// Clear store (tests).
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.inner.lock().expect("policy store poisoned").clear();
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
        let bundle = store.publish_raw("org-raw", "5", payload);
        assert_eq!(bundle.version, "5");
        assert!(store.verify_and_apply("org-raw", &bundle).is_ok());
    }
}
