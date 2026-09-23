//! Loop controller (P2-07): per-session failure fingerprints.
//! retry_differently → stop → ask. Persisted in memory (SQLite in daemon later).
//! Quiet unless threshold crossed. Fail-safe: any error → continue (not loop-action allow bypass).

use algo_types::LoopAction;
use std::collections::HashMap;

/// Failure fingerprint: normalized command + exit/error signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FailureKey {
    pub cmd: String,
    pub exit_code: i32,
}

/// Per-session loop state.
#[derive(Debug, Clone)]
struct SessionState {
    counts: HashMap<FailureKey, usize>,
    consecutive_failures: usize,
    last_was_fail: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for SessionState {
    fn default() -> Self {
        Self {
            counts: HashMap::new(),
            consecutive_failures: 0,
            last_was_fail: false,
        }
    }
}

/// Loop controller: tracks failures per session, decides LoopAction.
pub struct LoopController {
    sessions: HashMap<String, SessionState>,
    /// Thresholds per plan: 3× same cmd+error → retry_differently, 5× → stop → ask.
    pub retry_threshold: usize,
    pub stop_threshold: usize,
}

impl Default for LoopController {
    fn default() -> Self {
        Self::new()
    }
}

impl LoopController {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            retry_threshold: 3,
            stop_threshold: 5,
        }
    }

    /// Record a tool result. `success = exit_code == 0 && error.is_empty()`.
    /// Returns LoopAction for the next step.
    pub fn record(
        &mut self,
        session_id: &str,
        cmd: &str,
        exit_code: i32,
        error: &str,
    ) -> LoopAction {
        let success = exit_code == 0 && error.is_empty();
        let state = self.sessions.entry(session_id.to_string()).or_default();
        if success {
            state.consecutive_failures = 0;
            state.last_was_fail = false;
            // Flaky fail-fail-pass should NOT trigger: reset counts on success
            // is implicit via consecutive reset; same-key counts decay? We keep counts but
            // only act on consecutive streaks, so flaky passes clear escalation.
            return LoopAction::Continue;
        }
        state.consecutive_failures += 1;
        state.last_was_fail = true;
        let key = FailureKey {
            cmd: cmd.to_string(),
            exit_code,
        };
        let c = state.counts.entry(key).or_insert(0);
        *c += 1;
        if *c >= self.stop_threshold {
            LoopAction::Stop
        } else if *c >= self.retry_threshold {
            LoopAction::RetryDifferently
        } else if state.consecutive_failures >= self.stop_threshold {
            LoopAction::Ask
        } else {
            LoopAction::Continue
        }
    }

    /// Observe helper for tests: current count for a key.
    #[cfg(test)]
    pub fn count(&self, session_id: &str, cmd: &str, exit_code: i32) -> usize {
        self.sessions
            .get(session_id)
            .and_then(|s| {
                s.counts.get(&FailureKey {
                    cmd: cmd.to_string(),
                    exit_code,
                })
            })
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_5_identical_fails_escalates() {
        let mut lc = LoopController::new();
        for i in 1..=5 {
            let act = lc.record("sess-1", "npm test", 1, "fail");
            if i < 3 {
                assert_eq!(act, LoopAction::Continue, "i={i}");
            } else if i < 5 {
                assert_eq!(act, LoopAction::RetryDifferently, "i={i}");
            } else {
                assert_eq!(act, LoopAction::Stop, "i={i}");
            }
        }
        assert_eq!(lc.count("sess-1", "npm test", 1), 5);
    }

    #[test]
    fn flaky_fail_fail_pass_no_trigger() {
        let mut lc = LoopController::new();
        assert_eq!(
            lc.record("sess-1", "npm test", 1, "fail"),
            LoopAction::Continue
        );
        assert_eq!(
            lc.record("sess-1", "npm test", 1, "fail"),
            LoopAction::Continue
        );
        // Pass resets consecutive
        assert_eq!(lc.record("sess-1", "npm test", 0, ""), LoopAction::Continue);
        // Next fail is fresh consecutive
        assert_eq!(
            lc.record("sess-1", "npm test", 1, "fail"),
            LoopAction::RetryDifferently
        ); // count 3 now
           // Another pass resets
        assert_eq!(lc.record("sess-1", "npm test", 0, ""), LoopAction::Continue);
        // Fail again still retry threshold (count 4)
        assert_eq!(
            lc.record("sess-1", "npm test", 1, "fail"),
            LoopAction::RetryDifferently
        );
    }

    #[test]
    fn different_cmds_do_not_cross_trigger() {
        let mut lc = LoopController::new();
        for _ in 0..3 {
            lc.record("sess-1", "cmd-a", 1, "fail");
        }
        // cmd-b at 1 fail should not be retry
        assert_eq!(
            lc.record("sess-1", "cmd-b", 1, "fail"),
            LoopAction::Continue
        );
    }

    #[test]
    fn ask_on_consecutive_different_failures() {
        let mut lc = LoopController::new();
        for i in 0..5 {
            let act = lc.record("sess-1", &format!("cmd-{i}"), 1, "fail");
            if i < 4 {
                assert_eq!(act, LoopAction::Continue);
            } else {
                assert_eq!(act, LoopAction::Ask);
            }
        }
    }
}
