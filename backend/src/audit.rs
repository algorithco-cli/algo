//! Audit ingest: redacted_event, decision, latency — no source field.
//! Fails closed if not redacted (secret pattern via algo-redact-style scan).

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Allowed decisions (mirrors proto Action). Unknown → ask downstream, but ingest validates.
const ALLOWED_DECISIONS: &[&str] = &["allow", "deny", "ask", "ALLOW", "DENY", "ASK"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditRecord {
    pub redacted_event: String,
    pub decision: String,
    pub latency_ms: i64,
    pub trace_id: String,
    pub org_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    NotRedacted(String),
    MissingField(String),
    InvalidDecision(String),
    InvalidLatency(String),
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRedacted(s) => write!(f, "not redacted (secret detected): {s}"),
            Self::MissingField(s) => write!(f, "missing field: {s}"),
            Self::InvalidDecision(s) => write!(f, "invalid decision: {s}"),
            Self::InvalidLatency(s) => write!(f, "invalid latency: {s}"),
        }
    }
}
impl std::error::Error for AuditError {}

static AUDIT_STORE: OnceLock<Mutex<Vec<AuditRecord>>> = OnceLock::new();
static WAL_QUEUE: OnceLock<Mutex<VecDeque<AuditRecord>>> = OnceLock::new();

fn audit_store() -> &'static Mutex<Vec<AuditRecord>> {
    AUDIT_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

fn wal_queue() -> &'static Mutex<VecDeque<AuditRecord>> {
    WAL_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Very small secret scanner — mirrors `algo-redact` literals+regex for MVP.
/// If any pattern matches, payload is considered NOT redacted.
fn contains_secret(s: &str) -> Option<String> {
    // Fast literal pre-filter via regex set below; keep simple for MVP.
    let patterns: &[(&str, &str)] = &[
        (r"\bAKIA[0-9A-Z]{16}\b", "aws_key"),
        (r"\bghp_[A-Za-z0-9]{20,}\b", "github_pat"),
        (r"\bgho_[A-Za-z0-9]{20,}\b", "github_oauth"),
        (r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b", "slack_token"),
        (r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----", "private_key"),
        (r"\bsk-(live|test)-[A-Za-z0-9]{16,}\b", "vendor_sk"),
        (r"\bAIza[A-Za-z0-9_-]{35}\b", "google_api"),
        (
            r"\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b",
            "jwt",
        ),
        (
            r#"(?i)\b(password|passwd|pwd|token|secret)\b\s*[:=]\s*['"]?[^'"\s,}]{4,}"#,
            "credential_assignment",
        ),
    ];
    for (pat, kind) in patterns {
        let re = Regex::new(pat).expect("valid regex");
        if re.is_match(s) {
            return Some((*kind).to_string());
        }
    }
    // High-entropy dense token heuristic (len>=24, 3+ char classes, entropy>4.5) — simplified check.
    // We look for long alphanumeric-ish tokens not containing <REDACTED.
    // For MVP, scan for 24+ char base64-like substrings without spaces.
    let re_long = Regex::new(r"[A-Za-z0-9+/=_-]{24,}").expect("valid regex");
    for m in re_long.find_iter(s) {
        let tok = m.as_str();
        if tok.contains("<REDACTED") || tok.contains("example") || tok.contains("test") {
            continue;
        }
        // Quick char-class check: need upper, lower, digit.
        let has_upper = tok.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = tok.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = tok.chars().any(|c| c.is_ascii_digit());
        if has_upper && has_lower && has_digit && tok.len() >= 24 {
            // Entropy approx via distinct chars.
            let distinct = {
                let mut set = std::collections::HashSet::new();
                for b in tok.bytes() {
                    set.insert(b);
                }
                set.len() as f64
            };
            if distinct > 12.0 {
                return Some("high_entropy".to_string());
            }
        }
    }
    None
}

/// Public helper: true if payload would be considered redacted (i.e., no secret).
pub fn is_redacted_payload(s: &str) -> bool {
    contains_secret(s).is_none()
}

/// Ingest payload that may contain a spurious `source` field — drop it, log, validate.
/// `raw` is the JSON value received from daemon/agent.
///
/// Spec: `AuditIngest{redacted_event,decision,latency}` — no source field.
/// If `source` is present we drop it (privacy). If `redacted_event` contains secret, reject fails-closed.
pub fn ingest_audit(mut raw: serde_json::Value) -> Result<AuditRecord, AuditError> {
    // Drop source if present (privacy). Log via tracing for audit trail.
    if let Some(obj) = raw.as_object_mut() {
        if obj.contains_key("source") {
            tracing::warn!("audit ingest: dropping source field (privacy)");
            obj.remove("source");
        }
        // Also drop any nested source-ish keys like "source_bytes", "raw_source".
        for key in ["source_bytes", "raw_source", "original_payload"] {
            if obj.contains_key(key) {
                tracing::warn!("audit ingest: dropping {key} field");
                obj.remove(key);
            }
        }
    }

    let obj = raw
        .as_object()
        .ok_or_else(|| AuditError::MissingField("expected JSON object".to_string()))?;

    let redacted_event = obj
        .get("redacted_event")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuditError::MissingField("redacted_event".to_string()))?
        .to_string();

    let decision = obj
        .get("decision")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuditError::MissingField("decision".to_string()))?
        .to_string();

    let latency_ms = obj
        .get("latency_ms")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| AuditError::MissingField("latency_ms".to_string()))?;

    if latency_ms < 0 {
        return Err(AuditError::InvalidLatency(format!(
            "negative latency {latency_ms}"
        )));
    }

    // Validate decision (case-insensitive).
    if !ALLOWED_DECISIONS
        .iter()
        .any(|d| d.eq_ignore_ascii_case(&decision))
    {
        return Err(AuditError::InvalidDecision(decision));
    }

    // Secret check — fails closed if not redacted.
    if let Some(kind) = contains_secret(&redacted_event) {
        tracing::warn!("audit ingest rejected: secret pattern {kind} in redacted_event");
        return Err(AuditError::NotRedacted(kind));
    }

    // Trace/org ids optional but we generate/keep if present.
    let trace_id = obj
        .get("trace_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let org_id = obj
        .get("org_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let record = AuditRecord {
        redacted_event,
        decision,
        latency_ms,
        trace_id,
        org_id,
    };

    // Append to store + WAL queue (in-memory scaffold for MVP).
    audit_store()
        .lock()
        .expect("audit store poisoned")
        .push(record.clone());
    wal_queue()
        .lock()
        .expect("wal poisoned")
        .push_back(record.clone());

    Ok(record)
}

/// Get all audit records (for stats).
pub fn all_records() -> Vec<AuditRecord> {
    audit_store().lock().expect("audit store poisoned").clone()
}

/// Drain WAL queue (simulates apalis/Postgres queue consumer).
pub fn drain_wal() -> Vec<AuditRecord> {
    let mut q = wal_queue().lock().expect("wal poisoned");
    q.drain(..).collect()
}

/// Clear stores (tests).
pub fn clear_audit_store() {
    audit_store().lock().expect("audit store poisoned").clear();
    wal_queue().lock().expect("wal poisoned").clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_sync;
    use serde_json::json;

    #[test]
    fn ingest_ok_redacted() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({"redacted_event":"tool ls -la","decision":"allow","latency_ms":12,"trace_id":"t1"});
        let rec = ingest_audit(raw).expect("should ingest");
        assert_eq!(rec.decision, "allow");
        assert_eq!(rec.latency_ms, 12);
        assert_eq!(rec.trace_id, "t1");
    }

    #[test]
    fn ingest_drops_source_field() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"redacted payload",
            "decision":"deny",
            "latency_ms":5,
            "source":"should be dropped",
            "source_bytes":"also dropped",
            "trace_id":"t2"
        });
        let rec = ingest_audit(raw).expect("should ingest with source dropped");
        // Record itself never carries source; we prove the store has no source bytes.
        let all = all_records();
        assert_eq!(all.len(), 1);
        assert_eq!(rec.trace_id, "t2");
        // Ensure no secret leakage via source — the ingest should not retain source.
        // The redacted_event is clean, so no NotRedacted error.
        assert!(is_redacted_payload(&rec.redacted_event));
        // WAL also has same record without source.
        let wal = drain_wal();
        assert_eq!(wal.len(), 1);
        assert_eq!(wal[0].trace_id, "t2");
    }

    #[test]
    fn ingest_rejects_secret_in_payload_fails_closed() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"token ghp_12345678901234567890 leaked",
            "decision":"allow",
            "latency_ms":10
        });
        let err = ingest_audit(raw).unwrap_err();
        assert!(matches!(err, AuditError::NotRedacted(_)));
        // Nothing stored.
        assert!(all_records().is_empty());
        assert!(drain_wal().is_empty());
    }

    #[test]
    fn ingest_rejects_aws_key() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"key AKIAIOSFODNN7EXAMPLE",
            "decision":"ask",
            "latency_ms":7
        });
        let err = ingest_audit(raw).unwrap_err();
        assert!(matches!(err, AuditError::NotRedacted(_)));
    }

    #[test]
    fn ingest_rejects_private_key() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"-----BEGIN RSA PRIVATE KEY----- abc",
            "decision":"deny",
            "latency_ms":3
        });
        let err = ingest_audit(raw).unwrap_err();
        assert!(matches!(err, AuditError::NotRedacted(_)));
    }

    #[test]
    fn ingest_rejects_high_entropy_token() {
        let _guard = test_sync::lock();
        clear_audit_store();
        // High-entropy long token with upper/lower/digit.
        let secret = "aB3dEfGhIjKlMnOpQrStUvWxYz123456";
        let raw = json!({
            "redacted_event": format!("payload {}", secret),
            "decision":"allow",
            "latency_ms":4
        });
        // This may be flagged as high_entropy.
        let res = ingest_audit(raw);
        // If our heuristic flags it, it should be Err; if not, it's okay but we still test that clean data passes.
        // For deterministic, we assert that a known high-entropy is rejected.
        // The token above is 32 chars with mixed classes → should be flagged.
        assert!(res.is_err());
    }

    #[test]
    fn ingest_validates_decision() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"clean",
            "decision":"unknown_decision",
            "latency_ms":1
        });
        let err = ingest_audit(raw).unwrap_err();
        assert!(matches!(err, AuditError::InvalidDecision(_)));
    }

    #[test]
    fn ingest_validates_latency() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"clean",
            "decision":"allow",
            "latency_ms": -5
        });
        let err = ingest_audit(raw).unwrap_err();
        assert!(matches!(err, AuditError::InvalidLatency(_)));
    }

    #[test]
    fn proves_ask_on_not_redacted() {
        let _guard = test_sync::lock();
        // Fail-safe: not-redacted must not become allow.
        clear_audit_store();
        let raw = json!({
            "redacted_event":"AKIAIOSFODNN7EXAMPLE",
            "decision":"allow",
            "latency_ms":1
        });
        let err = ingest_audit(raw).unwrap_err();
        // Caller must map to ask; we prove the error is NotRedacted, not Ok.
        assert!(matches!(err, AuditError::NotRedacted(_)));
    }

    #[test]
    fn is_redacted_helper() {
        let _guard = test_sync::lock();
        assert!(is_redacted_payload("clean payload"));
        assert!(!is_redacted_payload("ghp_12345678901234567890"));
        assert!(is_redacted_payload("<REDACTED:GITHUB_PAT>"));
    }
}
