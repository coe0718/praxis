use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::state::SessionState;

pub const DEFAULT_LOOP_GUARD_LIMIT: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
    Allow,
    Blocked { consecutive_count: usize },
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
        if state.tool_invocation_hashes.len() > 10 {
            let drain = state.tool_invocation_hashes.len() - 10;
            state.tool_invocation_hashes.drain(0..drain);
        }

        let consecutive = state
            .tool_invocation_hashes
            .iter()
            .rev()
            .take_while(|existing| **existing == hash)
            .count();

        if consecutive >= limit {
            GuardDecision::Blocked {
                consecutive_count: consecutive,
            }
        } else {
            GuardDecision::Allow
        }
    }
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
                consecutive_count: 3
            }
        );
    }
}
