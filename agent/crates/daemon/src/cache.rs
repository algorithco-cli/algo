use algo_types::{Decision, SourceLevel};
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct Cache {
    inner: DashMap<String, (Decision, Instant)>,
    order: Mutex<VecDeque<String>>,
    ttl: Duration,
    max: usize,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            ttl: Duration::from_secs(24 * 3600),
            max: 10_000,
        }
    }

    #[cfg(test)]
    pub fn with_params(ttl: Duration, max: usize) -> Self {
        Self {
            inner: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            ttl,
            max,
        }
    }

    pub fn key_for(cmd: &str) -> String {
        let norm = algo_fingerprint::normalize(cmd);
        if norm.is_empty() || norm == "unparseable:nested" {
            blake3::hash(cmd.as_bytes()).to_hex().to_string()
        } else {
            algo_fingerprint::cache_key(&norm, "v0", "balanced")
        }
    }

    pub fn get(&self, cmd: &str) -> Option<Decision> {
        let k = Self::key_for(cmd);
        if let Some(entry) = self.inner.get(&k) {
            let (dec, instant) = entry.value();
            if instant.elapsed() < self.ttl {
                let mut d = dec.clone();
                d.source_level = SourceLevel::Cache as i32;
                return Some(d);
            }
            // expired
            drop(entry);
            self.inner.remove(&k);
        }
        None
    }

    pub fn insert(&self, cmd: &str, decision: Decision) {
        let k = Self::key_for(cmd);
        let mut evict: Option<String> = None;
        let mut is_update = false;
        {
            let mut order = self.order.lock().unwrap();
            if self.inner.contains_key(&k) {
                order.retain(|x| x != &k);
                is_update = true;
            } else if self.inner.len() >= self.max && !is_update {
                if let Some(old) = order.pop_front() {
                    evict = Some(old);
                }
            }
            // push new key to back (will be inserted after possible eviction)
            order.push_back(k.clone());
        }
        if let Some(old) = evict {
            self.inner.remove(&old);
        } else if !is_update && self.inner.len() >= self.max {
            // fallback: order was empty but inner still full (should not happen with proper order tracking)
            // collect keys without holding order lock
            let keys: Vec<String> = self.inner.iter().map(|e| e.key().clone()).collect();
            if let Some(first) = keys.into_iter().next() {
                self.inner.remove(&first);
            }
        }
        self.inner.insert(k, (decision, Instant::now()));
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&self) {
        self.inner.clear();
        if let Ok(mut order) = self.order.lock() {
            order.clear();
        }
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use algo_types::{Action, SourceLevel};

    fn dummy_decision(action: Action) -> Decision {
        Decision {
            action: action as i32,
            reason: "test".into(),
            confidence_0_1: 0.9,
            source_level: SourceLevel::Jev as i32,
            latency_ms: 5,
            policy_version: "v0".into(),
            trace_id: "t1".into(),
        }
    }

    #[test]
    fn cache_hit_returns_same() {
        let cache = Cache::new();
        let cmd = "ls -la /tmp/foo";
        let dec = dummy_decision(Action::Allow);
        cache.insert(cmd, dec.clone());
        let hit = cache.get(cmd).expect("should hit");
        // source should be Cache on hit
        assert_eq!(hit.source_level, SourceLevel::Cache as i32);
        // action preserved
        assert_eq!(hit.action, Action::Allow as i32);
        // key equivalence after fingerprint normalize
        let variant = "ls -la /tmp/bar";
        // different path normalizes to same? /tmp/foo vs /tmp/bar both become <PATH> so same key
        let hit2 = cache.get(variant);
        assert!(hit2.is_some(), "fingerprint should treat /tmp/* as same");
    }

    #[test]
    fn cache_ttl_expires() {
        let cache = Cache::with_params(Duration::from_millis(10), 100);
        let cmd = "echo hello";
        cache.insert(cmd, dummy_decision(Action::Allow));
        assert!(cache.get(cmd).is_some());
        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.get(cmd).is_none(), "should expire after TTL");
    }

    #[test]
    fn cache_max_evicts() {
        let cache = Cache::with_params(Duration::from_secs(3600), 2);
        cache.insert("cmd1", dummy_decision(Action::Allow));
        cache.insert("cmd2", dummy_decision(Action::Allow));
        assert_eq!(cache.len(), 2);
        cache.insert("cmd3", dummy_decision(Action::Deny));
        assert_eq!(cache.len(), 2, "should evict to keep max");
        // at least one of the three is gone, but we have 2 left
    }

    #[test]
    fn blake3_fallback_for_unparseable() {
        let cmd = "sh -c 'sh -c \"echo hi\"'";
        // normalize returns unparseable:nested
        assert_eq!(algo_fingerprint::normalize(cmd), "unparseable:nested");
        let k = Cache::key_for(cmd);
        assert_eq!(k, blake3::hash(cmd.as_bytes()).to_hex().to_string());
        let cache = Cache::new();
        cache.insert(cmd, dummy_decision(Action::Ask));
        assert!(cache.get(cmd).is_some());
    }
}
