use super::patterns::SessionRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriftStatus {
    InsufficientData,
    Stable,
    Regressed,
    Improving,
}

impl DriftStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InsufficientData => "insufficient_data",
            Self::Stable => "stable",
            Self::Regressed => "regressed",
            Self::Improving => "improving",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DriftReport {
    pub status: DriftStatus,
    pub recent_score: f32,
    pub baseline_score: f32,
}

pub(super) fn detect_drift(sessions: &[SessionRow], window: usize) -> DriftReport {
    if sessions.len() < window.saturating_mul(2) || window == 0 {
        return DriftReport {
            status: DriftStatus::InsufficientData,
            recent_score: 0.0,
            baseline_score: 0.0,
        };
    }

    let recent = &sessions[..window];
    let baseline = &sessions[window..window * 2];
    let recent_score = score(recent);
    let baseline_score = score(baseline);
    let delta = recent_score - baseline_score;

    let status = if delta >= 0.20 {
        DriftStatus::Regressed
    } else if delta <= -0.20 {
        DriftStatus::Improving
    } else {
        DriftStatus::Stable
    };

    DriftReport {
        status,
        recent_score,
        baseline_score,
    }
}

fn score(sessions: &[SessionRow]) -> f32 {
    let total = sessions.len() as f32;
    let review_rate =
        sessions.iter().filter(|session| session.reviewer_failures > 0).count() as f32 / total;
    let eval_rate =
        sessions.iter().filter(|session| session.eval_failures > 0).count() as f32 / total;
    let loop_rate = sessions
        .iter()
        .filter(|session| session.outcome == "blocked_loop_guard")
        .count() as f32
        / total;
    (review_rate * 0.5) + (eval_rate * 0.3) + (loop_rate * 0.2)
}

#[cfg(test)]
mod tests {
    use super::{DriftStatus, detect_drift};
    use crate::argus::patterns::SessionRow;

    #[test]
    fn detects_regression_against_baseline_window() {
        let baseline = (0..5).map(|day| session(day, false, false, "goal_selected"));
        let recent = (5..10).map(|day| session(day, true, true, "blocked_loop_guard"));
        let sessions = recent.chain(baseline).collect::<Vec<_>>();

        let report = detect_drift(&sessions, 5);
        assert_eq!(report.status, DriftStatus::Regressed);
        assert!(report.recent_score > report.baseline_score);
    }

    fn session(day: i64, review_failed: bool, eval_failed: bool, outcome: &str) -> SessionRow {
        SessionRow {
            day,
            outcome: outcome.to_string(),
            reviewer_failures: i64::from(review_failed),
            eval_failures: i64::from(eval_failed),
            repeated_reads_avoided: 0,
            selected_goal_id: None,
            selected_goal_title: None,
            selected_task: None,
        }
    }
}
