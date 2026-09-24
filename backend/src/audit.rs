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

/// Ingest size caps (fail-closed; prevents OOM via unbounded stores).
pub const MAX_EVENT_LEN: usize = 16 * 1024;
pub const MAX_ID_LEN: usize = 128;
pub const MAX_LATENCY_MS: i64 = 3_600_000;
pub const MAX_ORG_LEN: usize = 128;

static AUDIT_STORE: OnceLock<Mutex<Vec<AuditRecord>>> = OnceLock::new();
static WAL_QUEUE: OnceLock<Mutex<VecDeque<AuditRecord>>> = OnceLock::new();
static SECRET_SET: OnceLock<regex::RegexSet> = OnceLock::new();
static SECRET_LONG: OnceLock<Regex> = OnceLock::new();

fn audit_store() -> &'static Mutex<Vec<AuditRecord>> {
    AUDIT_STORE.get_or_init(|| Mutex::new(Vec::new()))
}

fn wal_queue() -> &'static Mutex<VecDeque<AuditRecord>> {
    WAL_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn lock_audit_store() -> std::sync::MutexGuard<'static, Vec<AuditRecord>> {
    match audit_store().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("audit store mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

fn lock_wal_queue() -> std::sync::MutexGuard<'static, VecDeque<AuditRecord>> {
    match wal_queue().lock() {
        Ok(g) => g,
        Err(poisoned) => {
            tracing::warn!("wal queue mutex poisoned; recovering inner");
            poisoned.into_inner()
        }
    }
}

const SECRET_PATTERNS: &[&str] = &[
    r"\bAKIA[0-9A-Z]{16}\b",
    r"\bghp_[A-Za-z0-9]{20,}\b",
    r"\bgho_[A-Za-z0-9]{20,}\b",
    r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b",
    r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----",
    r"\bsk-(live|test)-[A-Za-z0-9]{16,}\b",
    r"\bAIza[A-Za-z0-9_-]{35}\b",
    r"\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b",
    r#"(?i)\b(password|passwd|pwd|token|secret)\b\s*[:=]\s*['"]?[^'"\s,}]{4,}"#,
];

const SECRET_KINDS: &[&str] = &[
    "aws_key",
    "github_pat",
    "github_oauth",
    "slack_token",
    "private_key",
    "vendor_sk",
    "google_api",
    "jwt",
    "credential_assignment",
];

fn secret_set() -> &'static regex::RegexSet {
    SECRET_SET
        .get_or_init(|| regex::RegexSet::new(SECRET_PATTERNS).expect("valid secret regex set"))
}

fn secret_long_re() -> &'static Regex {
    SECRET_LONG.get_or_init(|| Regex::new(r"[A-Za-z0-9+/=_-]{24,}").expect("valid regex"))
}

/// Very small secret scanner — mirrors `algo-redact` literals+regex for MVP.
/// If any pattern matches, payload is considered NOT redacted.
/// Regexes are compiled once (static) to meet L2 latency budgets.
fn contains_secret(s: &str) -> Option<String> {
    let set = secret_set();
    if let Some(idx) = set.matches(s).into_iter().next() {
        return Some(SECRET_KINDS[idx].to_string());
    }
    // High-entropy dense token heuristic (len>=24, 3+ char classes, entropy>4.5) — simplified check.
    // We look for long alphanumeric-ish tokens not containing <REDACTED.
    // For MVP, scan for 24+ char base64-like substrings without spaces.
    let re_long = secret_long_re();
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
#[allow(dead_code)]
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
    if redacted_event.len() > MAX_EVENT_LEN {
        return Err(AuditError::InvalidLatency(format!(
            "redacted_event too large ({} > {MAX_EVENT_LEN})",
            redacted_event.len()
        )));
    }

    let decision = obj
        .get("decision")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AuditError::MissingField("decision".to_string()))?
        .to_string();
    if decision.len() > MAX_ID_LEN {
        return Err(AuditError::InvalidDecision("decision too long".to_string()));
    }

    // Strict latency typing: floats/strings/u64-overflow fail closed.
    let latency_val = obj
        .get("latency_ms")
        .ok_or_else(|| AuditError::MissingField("latency_ms".to_string()))?;
    let latency_ms = latency_val.as_i64().ok_or_else(|| {
        AuditError::InvalidLatency("latency_ms must be integer milliseconds".to_string())
    })?;

    if !(0..=MAX_LATENCY_MS).contains(&latency_ms) {
        return Err(AuditError::InvalidLatency(format!(
            "latency out of range 0..={MAX_LATENCY_MS}"
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
    // Client trace_ids are length-capped; oversized values fail closed to
    // prevent store/queue amplification.
    let trace_id = match obj.get("trace_id").and_then(|v| v.as_str()) {
        Some(s) => {
            if s.len() > MAX_ID_LEN {
                return Err(AuditError::MissingField("trace_id too long".to_string()));
            }
            // Control characters / newlines are rejected (log/header injection).
            if s.chars().any(|c| c.is_control()) {
                return Err(AuditError::MissingField("trace_id invalid".to_string()));
            }
            s.to_string()
        }
        None => uuid::Uuid::new_v4().to_string(),
    };
    let org_id = match obj.get("org_id").and_then(|v| v.as_str()) {
        Some(s) => {
            if s.len() > MAX_ORG_LEN || s.trim().is_empty() {
                return Err(AuditError::MissingField("org_id invalid".to_string()));
            }
            Some(s.to_string())
        }
        None => None,
    };

    let record = AuditRecord {
        redacted_event,
        decision,
        latency_ms,
        trace_id,
        org_id,
    };

    // Append to store + WAL queue (in-memory scaffold for MVP).
    // Bounded: drop oldest WAL entries beyond cap to avoid unbounded memory
    // growth (fail-safe: audit store keeps full history for MVP, WAL is bounded).
    const WAL_CAP: usize = 10_000;
    lock_audit_store().push(record.clone());
    {
        let mut q = lock_wal_queue();
        if q.len() >= WAL_CAP {
            q.pop_front();
        }
        q.push_back(record.clone());
    }

    Ok(record)
}

/// Get all audit records (for stats).
pub fn all_records() -> Vec<AuditRecord> {
    lock_audit_store().clone()
}

/// Drain WAL queue (simulates apalis/Postgres queue consumer).
pub fn drain_wal() -> Vec<AuditRecord> {
    lock_wal_queue().drain(..).collect()
}

/// Clear stores (tests).
#[allow(dead_code)]
pub fn clear_audit_store() {
    lock_audit_store().clear();
    lock_wal_queue().clear();
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

    #[test]
    fn proves_ask_on_oversize_event() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let big = "x".repeat(MAX_EVENT_LEN + 1);
        let raw = json!({"redacted_event": big, "decision":"allow","latency_ms":1});
        assert!(ingest_audit(raw).is_err());
        assert!(all_records().is_empty());
    }

    #[test]
    fn proves_ask_on_bad_latency_type() {
        let _guard = test_sync::lock();
        clear_audit_store();
        for raw in [
            json!({"redacted_event":"clean","decision":"allow","latency_ms":12.5}),
            json!({"redacted_event":"clean","decision":"allow","latency_ms":"fast"}),
            json!({"redacted_event":"clean","decision":"allow","latency_ms":i64::MAX}),
        ] {
            assert!(ingest_audit(raw).is_err());
        }
        assert!(all_records().is_empty());
    }
}
