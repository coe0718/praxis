use crate::identity::Goal;

use super::ProviderRequest;

pub(super) fn request_for_ask(prompt: &str) -> ProviderRequest {
    ProviderRequest {
        phase: "ask",
        system: "You are Praxis, a careful personal AI agent. Answer the operator directly and briefly. Do not claim that long-lived state, background work, or external actions changed unless the prompt explicitly says they already did.",
        input: format!("Operator question or one-shot request:\n{prompt}"),
        max_output_tokens: 220,
    }
}

pub(super) fn request_for_plan(goal: Option<&Goal>, task: Option<&str>) -> ProviderRequest {
    ProviderRequest {
        phase: "decide",
        system: "You are Praxis, a careful personal AI agent. Respond with one concise action summary describing the next safe step.",
        input: render_target(goal, task),
        max_output_tokens: 180,
    }
}

pub(super) fn request_for_finalize(
    planned_summary: &str,
    goal: Option<&Goal>,
    task: Option<&str>,
) -> ProviderRequest {
    ProviderRequest {
        phase: "act",
        system: "You are Praxis in the act phase. Write one concise operator-facing progress note. Do not claim external actions happened unless explicitly stated in the prompt.",
        input: format!(
            "Task context:\n{}\n\nPlanned summary:\n{}\n\nReturn a single concise status update.",
            render_target(goal, task),
            planned_summary
        ),
        max_output_tokens: 180,
    }
}

pub(super) fn render_stub_summary(goal: Option<&Goal>, task: Option<&str>) -> String {
    if let Some(task) = task {
        format!("Stub backend accepted task \"{task}\" for deferred execution.")
    } else if let Some(goal) = goal {
        format!(
            "Stub backend prepared goal {}: {} with safe internal maintenance only.",
            goal.id, goal.title
        )
    } else {
        "Stub backend performed idle maintenance because no task or open goal was available."
            .to_string()
    }
}

pub(super) fn render_stub_answer(prompt: &str) -> String {
    format!("Stub backend answered without creating a session: {prompt}")
}

fn render_target(goal: Option<&Goal>, task: Option<&str>) -> String {
    if let Some(task) = task {
        format!("Operator task: {task}")
    } else if let Some(goal) = goal {
        format!("Goal {}: {}", goal.id, goal.title)
    } else {
        "No explicit task or goal is active. Produce a safe idle-maintenance summary.".to_string()
    }
}
