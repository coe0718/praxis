use super::{ArgusReport, DriftReport, DriftStatus, RepeatedWorkPattern};

pub(super) struct DirectiveInputs<'a> {
    pub session_count: usize,
    pub review_failures: i64,
    pub eval_failures: i64,
    pub loop_guard_blocks: i64,
    pub waiting_sessions: usize,
    pub repeated_reads_avoided: i64,
    pub drift: &'a DriftReport,
    pub repeated_work: &'a [RepeatedWorkPattern],
    pub token_hotspots: &'a [(String, String, i64)],
}

pub fn render(report: &ArgusReport) -> String {
    let mut lines = vec![
        "argus: ready".to_string(),
        format!("sessions_analyzed: {}", report.session_count),
        format!("review_failures: {}", report.review_failures),
        format!("eval_failures: {}", report.eval_failures),
        format!("loop_guard_blocks: {}", report.loop_guard_blocks),
        format!("waiting_sessions: {}", report.waiting_sessions),
        format!("repeated_reads_avoided: {}", report.repeated_reads_avoided),
        format!(
            "drift_status: {} recent={:.2} baseline={:.2}",
            report.drift.status.as_str(),
            report.drift.recent_score,
            report.drift.baseline_score
        ),
    ];

    if report.repeated_work.is_empty() {
        lines.push("repeated_work: none".to_string());
    } else {
        lines.push("repeated_work:".to_string());
        lines.extend(report.repeated_work.iter().map(|pattern| {
            format!(
                "- {} sessions={} days={} latest={}",
                pattern.label, pattern.sessions, pattern.distinct_days, pattern.latest_outcome
            )
        }));
    }

    if report.failure_clusters.is_empty() {
        lines.push("failure_clusters: none".to_string());
    } else {
        lines.push("failure_clusters:".to_string());
        lines.extend(
            report.failure_clusters.iter().map(|(name, count)| format!("- {name} x{count}")),
        );
    }

    if report.token_hotspots.is_empty() {
        lines.push("token_hotspots: none".to_string());
    } else {
        lines.push("token_hotspots:".to_string());
        lines.extend(
            report
                .token_hotspots
                .iter()
                .map(|(phase, provider, tokens)| format!("- {phase}/{provider} tokens={tokens}")),
        );
    }

    lines.push("directives:".to_string());
    lines.extend(report.directives.iter().map(|line| format!("- {line}")));
    lines.join("\n")
}

pub(super) fn directives(inputs: DirectiveInputs<'_>) -> Vec<String> {
    let mut directives = Vec::new();
    if inputs.drift.status == DriftStatus::Regressed {
        directives.push(
            "Recent quality is drifting downward; pause ambitious changes and repair the recurring failure path first."
                .to_string(),
        );
    }
    if inputs.review_failures > 0 {
        directives.push(
            "Tighten completion discipline: rehearse or self-check work before Reflect hands it to the reviewer."
                .to_string(),
        );
    }
    if inputs.eval_failures > 0 {
        directives.push(
            "Protect operator-specific behavior: inspect recent eval regressions before changing runtime habits or prompts."
                .to_string(),
        );
    }
    if inputs.loop_guard_blocks > 0 {
        directives.push(
            "Reduce retry thrash: diversify tool plans earlier instead of repeating identical invocations."
                .to_string(),
        );
    }
    if inputs.waiting_sessions > 0 {
        directives.push(
            "Clarify blocked goals: promote prerequisites or satisfy wake conditions so sessions do not idle behind dependencies."
                .to_string(),
        );
    }
    if inputs.repeated_work.iter().any(|pattern| pattern.distinct_days >= 2) {
        directives.push(
            "Recurring work is resurfacing across multiple days; promote it into automation, a parent goal, or the learning runtime."
                .to_string(),
        );
    }
    if let Some((phase, _, _)) = inputs.token_hotspots.first() {
        directives.push(format!(
            "Trim the {phase} phase first when chasing token savings; it is the hottest recent context path."
        ));
    }
    if inputs.repeated_reads_avoided > 0 {
        directives.push(
            "Expand anatomy coverage around frequently revisited files so more rereads can be replaced with summaries."
                .to_string(),
        );
    }
    if directives.is_empty() {
        directives.push(format!(
            "Recent sessions look stable across the last {} runs. Keep the current operating pattern and watch for drift.",
            inputs.session_count
        ));
    }
    directives
}
