//! Speculative execution — compare multiple rehearsed plan branches before
//! committing to the safest or highest-yield action.
//!
//! For complex goals with multiple plausible approaches, the Act phase can
//! rehearse two or more lightweight branches, compare them against success
//! criteria and trust constraints, and only then commit to the branch most
//! likely to succeed.  This is especially valuable when the cheapest safe move
//! is "try two narrow approaches in rehearsal, not one large irreversible one."
//!
//! # How it works
//!
//! 1. The caller supplies N [`SpeculativeBranch`]es — candidate plans, each
//!    with a description and the plan text produced by a backend.
//! 2. [`select_branch`] scores each branch against a set of success criteria
//!    (keyword / phrase checks against the plan text) and optional trust
//!    constraints (phrases that, if present, penalise a branch).
//! 3. The highest-scoring branch is returned as the winner.  Ties are broken
//!    by the branch's position in the input list (earlier = preferred).
//!
//! The scoring model is intentionally lightweight: it is designed as a
//! first-class primitive that a future evaluator backend can replace with
//! semantic scoring.

use serde::{Deserialize, Serialize};

// ── Branch ────────────────────────────────────────────────────────────────────

/// A candidate action plan produced by rehearsing one approach.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeBranch {
    /// Short identifier (e.g. `"branch-a"`, `"branch-b"`).
    pub id: String,
    /// Human-readable label for the approach.
    pub description: String,
    /// The plan text — summary of what this branch would do.
    pub plan_text: String,
}

impl SpeculativeBranch {
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        plan_text: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            plan_text: plan_text.into(),
        }
    }
}

// ── Scoring ───────────────────────────────────────────────────────────────────

/// Score assigned to a branch during selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchScore {
    pub branch_id: String,
    /// Number of success criteria matched.
    pub criteria_matched: usize,
    /// Total number of success criteria evaluated.
    pub criteria_total: usize,
    /// Number of trust constraints that fired (penalises the branch).
    pub trust_violations: usize,
    /// Composite score: criteria_matched − trust_violations.  May be negative.
    pub score: i32,
}

impl BranchScore {
    fn compute(
        branch: &SpeculativeBranch,
        success_criteria: &[String],
        trust_constraints: &[String],
    ) -> Self {
        let text_lower = branch.plan_text.to_lowercase();
        let criteria_matched = success_criteria
            .iter()
            .filter(|c| text_lower.contains(&c.to_lowercase()))
            .count();
        let trust_violations = trust_constraints
            .iter()
            .filter(|c| text_lower.contains(&c.to_lowercase()))
            .count();
        let score = criteria_matched as i32 - trust_violations as i32;
        BranchScore {
            branch_id: branch.id.clone(),
            criteria_matched,
            criteria_total: success_criteria.len(),
            trust_violations,
            score,
        }
    }
}

// ── Result ────────────────────────────────────────────────────────────────────

/// The outcome of a speculative selection pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeculativeResult {
    /// The selected branch.
    pub winner: SpeculativeBranch,
    /// Scores for all evaluated branches, in input order.
    pub scores: Vec<BranchScore>,
    /// Human-readable rationale for the selection.
    pub rationale: String,
}

// ── Selection ─────────────────────────────────────────────────────────────────

/// Evaluate all branches against the given criteria and return the winner.
///
/// `success_criteria` — phrases that should appear in the plan text to
///   signal it covers the required ground.
/// `trust_constraints` — phrases whose presence penalises a branch (e.g.
///   "delete production", "force push").
///
/// Returns `None` if `branches` is empty.
pub fn select_branch(
    branches: Vec<SpeculativeBranch>,
    success_criteria: &[String],
    trust_constraints: &[String],
) -> Option<SpeculativeResult> {
    if branches.is_empty() {
        return None;
    }

    let scores: Vec<BranchScore> = branches
        .iter()
        .map(|b| BranchScore::compute(b, success_criteria, trust_constraints))
        .collect();

    // Pick highest score; ties resolved by earliest position.
    let best_idx = scores
        .iter()
        .enumerate()
        .fold((0usize, i32::MIN), |(best_i, best_s), (i, s)| {
            if s.score > best_s {
                (i, s.score)
            } else {
                (best_i, best_s)
            }
        })
        .0;

    let winner = branches[best_idx].clone();
    let best_score = &scores[best_idx];

    let rationale = if success_criteria.is_empty() && trust_constraints.is_empty() {
        format!("Selected {} (first branch, no criteria provided).", winner.id)
    } else {
        format!(
            "Selected {} — score {} ({}/{} criteria matched, {} trust violations).",
            winner.id,
            best_score.score,
            best_score.criteria_matched,
            best_score.criteria_total,
            best_score.trust_violations,
        )
    };

    Some(SpeculativeResult { winner, scores, rationale })
}

#[cfg(test)]
mod tests {
    use super::{SpeculativeBranch, select_branch};

    fn branch(id: &str, plan: &str) -> SpeculativeBranch {
        SpeculativeBranch::new(id, format!("Approach {id}"), plan)
    }

    #[test]
    fn selects_branch_with_most_criteria_matched() {
        let branches = vec![
            branch("a", "Run tests and check logs"),
            branch("b", "Run tests, check logs, and update docs"),
        ];
        let criteria = vec!["tests".to_string(), "logs".to_string(), "docs".to_string()];
        let result = select_branch(branches, &criteria, &[]).unwrap();
        assert_eq!(result.winner.id, "b");
        assert_eq!(result.scores[1].criteria_matched, 3);
    }

    #[test]
    fn trust_violations_penalise_branch() {
        let branches = vec![
            branch("safe", "Run tests carefully"),
            branch("risky", "Run tests and force push to production"),
        ];
        let criteria = vec!["tests".to_string()];
        let constraints = vec!["force push".to_string()];
        let result = select_branch(branches, &criteria, &constraints).unwrap();
        assert_eq!(result.winner.id, "safe");
        assert_eq!(result.scores[1].trust_violations, 1);
        assert_eq!(result.scores[1].score, 0); // 1 criteria - 1 violation
    }

    #[test]
    fn tie_broken_by_earliest_branch() {
        let branches = vec![branch("first", "check logs"), branch("second", "check logs")];
        let criteria = vec!["logs".to_string()];
        let result = select_branch(branches, &criteria, &[]).unwrap();
        assert_eq!(result.winner.id, "first");
    }

    #[test]
    fn returns_none_for_empty_branches() {
        assert!(select_branch(vec![], &[], &[]).is_none());
    }

    #[test]
    fn single_branch_always_wins() {
        let branches = vec![branch("only", "do the thing")];
        let result = select_branch(branches, &[], &[]).unwrap();
        assert_eq!(result.winner.id, "only");
    }

    #[test]
    fn rationale_is_informative() {
        let branches = vec![branch("a", "ship it")];
        let criteria = vec!["ship".to_string()];
        let result = select_branch(branches, &criteria, &[]).unwrap();
        assert!(result.rationale.contains("a"));
        assert!(result.rationale.contains("1/1"));
    }
}
