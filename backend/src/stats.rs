//! Stats aggregates from audit in-memory.

use serde::{Deserialize, Serialize};

use crate::audit::all_records;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stats {
    pub total: i64,
    pub allow: i64,
    pub deny: i64,
    pub ask: i64,
    pub avg_latency_ms: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            total: 0,
            allow: 0,
            deny: 0,
            ask: 0,
            avg_latency_ms: 0.0,
        }
    }
}

/// Aggregate stats for org (if Some) or all orgs (if None).
pub fn query_stats(org_id: Option<&str>) -> Stats {
    let records = all_records();
    let filtered: Vec<_> = records
        .into_iter()
        .filter(|r| {
            if let Some(filter_org) = org_id {
                if filter_org.is_empty() {
                    true
                } else {
                    r.org_id.as_deref() == Some(filter_org)
                }
            } else {
                true
            }
        })
        .collect();

    let total = filtered.len() as i64;
    if total == 0 {
        return Stats::default();
    }
    let mut allow = 0i64;
    let mut deny = 0i64;
    let mut ask = 0i64;
    let mut latency_sum: i64 = 0;
    for r in &filtered {
        match r.decision.to_ascii_lowercase().as_str() {
            "allow" => allow += 1,
            "deny" => deny += 1,
            "ask" => ask += 1,
            _ => ask += 1, // unknown → ask failsafe
        }
        latency_sum += r.latency_ms;
    }
    let avg_latency_ms = latency_sum as f64 / total as f64;
    Stats {
        total,
        allow,
        deny,
        ask,
        avg_latency_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{clear_audit_store, ingest_audit};
    use crate::test_sync;
    use serde_json::json;

    #[test]
    fn stats_empty() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let s = query_stats(None);
        assert_eq!(s.total, 0);
        assert_eq!(s.avg_latency_ms, 0.0);
    }

    #[test]
    fn stats_aggregates() {
        let _guard = test_sync::lock();
        clear_audit_store();
        for (decision, latency) in [("allow", 10), ("deny", 20), ("ask", 30), ("allow", 40)] {
            let raw = json!({
                "redacted_event": format!("event {decision}"),
                "decision": decision,
                "latency_ms": latency,
                "org_id":"org-1"
            });
            ingest_audit(raw).unwrap();
        }
        let s = query_stats(Some("org-1"));
        assert_eq!(s.total, 4);
        assert_eq!(s.allow, 2);
        assert_eq!(s.deny, 1);
        assert_eq!(s.ask, 1);
        assert!((s.avg_latency_ms - 25.0).abs() < 0.001);

        // Filter other org → 0.
        let s2 = query_stats(Some("org-2"));
        assert_eq!(s2.total, 0);

        // No filter → all.
        let s_all = query_stats(None);
        assert_eq!(s_all.total, 4);
    }

    #[test]
    fn stats_case_insensitive_decision() {
        let _guard = test_sync::lock();
        clear_audit_store();
        let raw = json!({
            "redacted_event":"clean",
            "decision":"ALLOW",
            "latency_ms":5
        });
        ingest_audit(raw).unwrap();
        let s = query_stats(None);
        assert_eq!(s.allow, 1);
    }
}
