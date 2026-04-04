use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::state::SessionState;

pub const DEFAULT_LOOP_GUARD_LIMIT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
    Allow,
    Blocked {
        consecutive_count: usize,
        pattern_len: usize,
    },
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LoopGuard;

impl LoopGuard {
    pub fn record(
        &self,
        state: &mut SessionState,
        invocation_key: &str,
        limit: usize,
    ) -> GuardDecision {
        let hash = hash_key(invocation_key);
        state.tool_invocation_hashes.push(hash.clone());
        if state.tool_invocation_hashes.len() > 12 {
            let drain = state.tool_invocation_hashes.len() - 12;
            state.tool_invocation_hashes.drain(0..drain);
        }

        for pattern_len in 1..=3 {
            let consecutive = repeated_pattern_count(&state.tool_invocation_hashes, pattern_len);
            if consecutive >= limit {
                return GuardDecision::Blocked {
                    consecutive_count: consecutive,
                    pattern_len,
                };
            }
        }

        GuardDecision::Allow
    }
}

fn repeated_pattern_count(history: &[String], pattern_len: usize) -> usize {
    if history.len() < pattern_len {
        return 0;
    }

    let pattern = &history[history.len() - pattern_len..];
    if pattern_len > 1 && pattern.iter().all(|entry| entry == &pattern[0]) {
        return 0;
    }

    let mut consecutive = 0;
    let mut end = history.len();
    while end >= pattern_len {
        let start = end - pattern_len;
        if history[start..end] == *pattern {
            consecutive += 1;
            end = start;
        } else {
            break;
        }
    }
    consecutive
}

fn hash_key(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::{DEFAULT_LOOP_GUARD_LIMIT, GuardDecision, LoopGuard};
    use crate::state::SessionState;

    #[test]
    fn blocks_repeated_invocations() {
        let mut state = SessionState::new(
            chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            None,
        );
        let guard = LoopGuard;

        assert_eq!(
            guard.record(&mut state, "shell:echo hi", DEFAULT_LOOP_GUARD_LIMIT),
            GuardDecision::Allow
        );
        assert_eq!(
            guard.record(&mut state, "shell:echo hi", DEFAULT_LOOP_GUARD_LIMIT),
            GuardDecision::Allow
        );
        assert_eq!(
            guard.record(&mut state, "shell:echo hi", DEFAULT_LOOP_GUARD_LIMIT),
            GuardDecision::Blocked {
                consecutive_count: 3,
                pattern_len: 1,
            }
        );
    }

    #[test]
    fn blocks_repeated_two_step_patterns() {
        let mut state = SessionState::new(
            chrono::Utc.with_ymd_and_hms(2026, 3, 31, 12, 0, 0).unwrap(),
            None,
        );
        let guard = LoopGuard;

        for invocation in ["tool:a", "tool:b", "tool:a", "tool:b", "tool:a"] {
            assert_eq!(
                guard.record(&mut state, invocation, DEFAULT_LOOP_GUARD_LIMIT),
                GuardDecision::Allow
            );
        }

        assert_eq!(
            guard.record(&mut state, "tool:b", DEFAULT_LOOP_GUARD_LIMIT),
            GuardDecision::Blocked {
                consecutive_count: 3,
                pattern_len: 2,
            }
        );
    }
}
