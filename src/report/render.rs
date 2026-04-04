use super::StatusReport;

pub fn render_status_report(report: &StatusReport) -> String {
    let mut lines = vec![
        "status: ready".to_string(),
        format!("instance: {}", report.instance_name),
        format!("backend: {}", report.backend),
        format!("profile: {}", report.profile),
        format!(
            "freeze_on_model_regression: {}",
            report.freeze_on_model_regression
        ),
        format!("data_dir: {}", report.data_dir),
        format!("phase: {}", report.phase),
        format!("last_outcome: {}", report.last_outcome),
        format!("repeated_reads_avoided: {}", report.repeated_reads_avoided),
        format!("anatomy_entries: {}", report.anatomy_entries),
        format!("pending_approvals: {}", report.pending_approvals),
        format!("pending_opportunities: {}", report.pending_opportunities),
        format!("registered_tools: {}", report.registered_tools),
        format!("drift_status: {}", report.drift_status),
        format!(
            "operational_memory: dnr={} bugs={}",
            report.operational_memory.do_not_repeat, report.operational_memory.known_bugs
        ),
        format!("event_count: {}", report.event_count),
        format!("canary_records: {}", report.canary_records),
    ];

    if let Some(selected_tool) = &report.selected_tool {
        lines.push(format!("selected_tool: {selected_tool}"));
    }
    if let Some(last_event) = &report.last_event {
        lines.push(format!("last_event: {}", last_event.kind));
    }
    if let Some(heartbeat) = &report.heartbeat {
        lines.push(format!(
            "heartbeat: phase={} updated_at={}",
            heartbeat.phase, heartbeat.updated_at
        ));
    }
    if let Some(session) = &report.last_session {
        lines.push(format!(
            "last_session: #{} {}",
            session.session_num, session.outcome
        ));
        lines.push(format!("last_session_ended_at: {}", session.ended_at));
    } else {
        lines.push("last_session: none".to_string());
    }
    if let Some(review_status) = &report.last_review_status {
        lines.push(format!("last_review: {review_status}"));
    }
    if let Some(eval) = &report.last_eval {
        lines.push(format!(
            "last_eval: passed={} failed={} skipped={} trust_failures={}",
            eval.passed, eval.failed, eval.skipped, eval.trust_failures
        ));
    }
    if let Some(usage) = &report.last_provider_usage {
        lines.push(format!(
            "last_provider: {} attempts={} failures={} tokens={} est_cost_usd={:.6}",
            usage.last_provider,
            usage.attempt_count,
            usage.failure_count,
            usage.tokens_used,
            usage.estimated_cost_micros as f64 / 1_000_000.0
        ));
    }
    if let Some(summary) = &report.last_token_summary {
        lines.push(format!(
            "last_tokens: total={} est_cost_usd={:.6}",
            summary.tokens_used,
            summary.estimated_cost_micros as f64 / 1_000_000.0
        ));
    }
    if let Some(hotspot) = &report.last_token_hotspot {
        lines.push(format!(
            "token_hotspot: {}/{} tokens={} est_cost_usd={:.6}",
            hotspot.phase,
            hotspot.provider,
            hotspot.tokens_used,
            hotspot.estimated_cost_micros as f64 / 1_000_000.0
        ));
    }
    if let Some(run) = &report.last_learning_run {
        lines.push(format!(
            "last_learning: processed={} changed={} opportunities={} completed_at={}",
            run.processed_sources, run.changed_sources, run.opportunities_created, run.completed_at
        ));
    }

    lines.join("\n")
}
