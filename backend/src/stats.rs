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
    /// Time series (Phase 1 contract). Empty unless `query_stats_series`.
    #[serde(default)]
    pub buckets: Vec<StatsBucket>,
    #[serde(default)]
    pub per_user: Vec<PerUserStats>,
    #[serde(default)]
    pub per_project: Vec<PerProjectStats>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatsBucket {
    pub bucket_start: String,
    pub total: i64,
    pub allow: i64,
    pub deny: i64,
    pub ask: i64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerUserStats {
    pub user_id: String,
    pub total: i64,
    pub allow: i64,
    pub deny: i64,
    pub ask: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerProjectStats {
    pub project: String,
    pub total: i64,
    pub allow: i64,
    pub deny: i64,
    pub ask: i64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            total: 0,
            allow: 0,
            deny: 0,
            ask: 0,
            avg_latency_ms: 0.0,
            buckets: Vec::new(),
            per_user: Vec::new(),
            per_project: Vec::new(),
            truncated: false,
        }
    }
}

fn bucket_of(decision: &str, bucket: &mut StatsBucket, latency_ms: i64, lat_sum: &mut i64) {
    bucket.total += 1;
    match decision {
        "allow" => bucket.allow += 1,
        "deny" => bucket.deny += 1,
        _ => bucket.ask += 1, // unknown → ask failsafe
    }
    *lat_sum = lat_sum.saturating_add(latency_ms);
}

/// Bucket granularity for the series extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Granularity {
    #[default]
    Day,
    Hour,
}

impl Granularity {
    /// Unknown values fall back to Day (contract; never an error).
    pub fn parse(s: Option<&str>) -> Self {
        match s.map(str::to_ascii_lowercase).as_deref() {
            Some("hour") => Self::Hour,
            _ => Self::Day,
        }
    }

    fn key(&self, ts: &chrono::DateTime<chrono::Utc>) -> String {
        match self {
            Self::Day => ts.format("%Y-%m-%d").to_string(),
            Self::Hour => ts.format("%Y-%m-%dT%H:00:00Z").to_string(),
        }
    }
}

/// Series query (mirrors proto QueryStatsRequest extension).
pub struct StatsSeriesQuery<'a> {
    pub org_id: Option<&'a str>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub granularity: Granularity,
    /// Max buckets (caller-capped). 0 = unlimited.
    pub limit: usize,
    /// Max per_user rows (caller-capped). 0 = unlimited.
    pub top_n: usize,
}

/// Aggregate stats + time series for org (if Some) or all orgs (None).
/// `per_project` is empty until project attribution lands in audit ingest.
/// Records with unparseable (never: server-set) timestamps are skipped.
pub fn query_stats_series(q: &StatsSeriesQuery<'_>) -> Stats {
    use std::collections::BTreeMap;
    let records = all_records();
    let mut buckets: BTreeMap<String, (StatsBucket, i64)> = BTreeMap::new();
    let mut users: BTreeMap<String, PerUserStats> = BTreeMap::new();
    let mut total = 0i64;
    let mut allow = 0i64;
    let mut deny = 0i64;
    let mut ask = 0i64;
    let mut latency_sum: i64 = 0;
    for r in &records {
        match q.org_id {
            Some("") => continue, // empty filter matches nothing (fail-closed)
            Some(filter_org) if r.org_id.as_deref() != Some(filter_org) => continue,
            _ => {}
        }
        let ts = chrono::DateTime::parse_from_rfc3339(&r.ingested_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok();
        let ts = match ts {
            Some(ts) => ts,
            None => continue,
        };
        if q.from.is_some_and(|from| ts < from) || q.to.is_some_and(|to| ts > to) {
            continue;
        }
        let decision = r.decision.to_ascii_lowercase();
        total += 1;
        match decision.as_str() {
            "allow" => allow += 1,
            "deny" => deny += 1,
            _ => ask += 1,
        }
        latency_sum = latency_sum.saturating_add(r.latency_ms);
        let key = q.granularity.key(&ts);
        let entry = buckets.entry(key.clone()).or_insert((
            StatsBucket {
                bucket_start: key,
                total: 0,
                allow: 0,
                deny: 0,
                ask: 0,
                avg_latency_ms: 0.0,
            },
            0,
        ));
        bucket_of(&decision, &mut entry.0, r.latency_ms, &mut entry.1);
        if let Some(user) = r.user_id.as_deref() {
            let row = users.entry(user.to_string()).or_insert(PerUserStats {
                user_id: user.to_string(),
                total: 0,
                allow: 0,
                deny: 0,
                ask: 0,
            });
            row.total += 1;
            match decision.as_str() {
                "allow" => row.allow += 1,
                "deny" => row.deny += 1,
                _ => row.ask += 1,
            }
        }
    }
    let avg_latency_ms = if total == 0 {
        0.0
    } else {
        latency_sum as f64 / total as f64
    };
    let mut bucket_list: Vec<StatsBucket> = buckets
        .into_values()
        .map(|(mut b, sum)| {
            b.avg_latency_ms = if b.total == 0 {
                0.0
            } else {
                sum as f64 / b.total as f64
            };
            b
        })
        .collect();
    // BTreeMap iterates ascending — newest last; dashboard plots in order.
    let mut truncated = false;
    if q.limit > 0 && bucket_list.len() > q.limit {
        // Keep the most recent buckets.
        bucket_list = bucket_list.split_off(bucket_list.len() - q.limit);
        truncated = true;
    }
    let mut user_list: Vec<PerUserStats> = users.into_values().collect();
    user_list.sort_by(|a, b| b.total.cmp(&a.total).then(a.user_id.cmp(&b.user_id)));
    if q.top_n > 0 && user_list.len() > q.top_n {
        user_list.truncate(q.top_n);
        truncated = true;
    }
    Stats {
        total,
        allow,
        deny,
        ask,
        avg_latency_ms,
        buckets: bucket_list,
        per_user: user_list,
        per_project: Vec::new(),
        truncated,
    }
}

/// Aggregate stats for org (if Some) or all orgs (if None).
/// Empty-string filter matches nothing (fail-closed; handlers reject it with 400).
/// Simple wrapper over [`query_stats_series`] without series (kept for
/// callers that only need totals).
#[allow(dead_code)]
pub fn query_stats(org_id: Option<&str>) -> Stats {
    query_stats_series(&StatsSeriesQuery {
        org_id,
        from: None,
        to: None,
        granularity: Granularity::Day,
        limit: 1,
        top_n: 0,
    })
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
