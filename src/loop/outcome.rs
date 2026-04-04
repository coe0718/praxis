use crate::{quality::EvalSummary, storage::ReviewStatus};

pub(super) fn final_outcome(
    initial: &str,
    review_status: ReviewStatus,
    eval_failures: usize,
) -> String {
    if review_status == ReviewStatus::Failed {
        "review_failed".to_string()
    } else if eval_failures > 0 {
        "eval_failed".to_string()
    } else {
        initial.to_string()
    }
}

pub(super) fn compose_summary(
    action_summary: &str,
    review_summary: &str,
    eval_summary: EvalSummary,
    findings: &[String],
) -> String {
    let mut summary = format!(
        "{action_summary} {review_summary} Evals passed={}, failed={}, skipped={}.",
        eval_summary.passed, eval_summary.failed, eval_summary.skipped
    );
    if !findings.is_empty() {
        summary.push_str(" Findings: ");
        summary.push_str(&findings.join(" | "));
    }
    summary
}
