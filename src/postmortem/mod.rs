use std::{fs, io::Write};

use anyhow::{Context, Result};

use crate::{
    paths::PraxisPaths,
    quality::{EvalOutcome, EvalSummary},
    storage::StoredSession,
};

pub fn append_postmortem(
    paths: &PraxisPaths,
    session: &StoredSession,
    outcome: &str,
    review_summary: &str,
    findings: &[String],
    eval_summary: EvalSummary,
    eval_results: &[EvalOutcome],
) -> Result<()> {
    if !should_record(outcome, eval_summary.failed) {
        return Ok(());
    }

    let path = paths.data_dir.join("POSTMORTEMS.md");
    let existed = path.exists();
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    if !existed {
        writeln!(file, "# Postmortems")?;
    }

    writeln!(file)?;
    writeln!(file, "## Session {} - {}", session.id, session.ended_at)?;
    writeln!(file, "- Outcome: {outcome}")?;
    writeln!(
        file,
        "- Goal: {}",
        session
            .selected_goal_id
            .as_deref()
            .zip(session.selected_goal_title.as_deref())
            .map(|(id, title)| format!("{id}: {title}"))
            .unwrap_or_else(|| "-".to_string())
    )?;
    writeln!(file, "- Task: {}", session.selected_task.as_deref().unwrap_or("-"))?;
    writeln!(file, "- Summary: {}", session.action_summary)?;
    writeln!(file, "- Review: {review_summary}")?;
    writeln!(
        file,
        "- Eval summary: passed={} failed={} skipped={} trust_failures={}",
        eval_summary.passed, eval_summary.failed, eval_summary.skipped, eval_summary.trust_failures
    )?;
    writeln!(file, "- Trigger: {}", trigger_label(outcome))?;
    writeln!(file, "### Findings")?;
    if findings.is_empty() {
        writeln!(file, "- No reviewer findings were recorded.")?;
    } else {
        for finding in findings {
            writeln!(file, "- {finding}")?;
        }
    }

    writeln!(file, "### Failed Evals")?;
    let mut wrote_eval = false;
    for eval in eval_results.iter().filter(|item| item.status.as_str() == "failed") {
        writeln!(file, "- {} ({})", eval.name, eval.eval_id)?;
        wrote_eval = true;
    }
    if !wrote_eval {
        writeln!(file, "- No eval failures were recorded.")?;
    }
    Ok(())
}

fn should_record(outcome: &str, eval_failures: usize) -> bool {
    matches!(outcome, "review_failed" | "eval_failed" | "blocked_loop_guard") || eval_failures > 0
}

fn trigger_label(outcome: &str) -> &'static str {
    match outcome {
        "review_failed" => "review gate failed",
        "eval_failed" => "operator eval failed",
        "blocked_loop_guard" => "loop guard stopped repeated behavior",
        _ => "unexpected runtime regression",
    }
}
