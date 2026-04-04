use std::fs;

use anyhow::{Result, bail};

use crate::{
    boundaries::{BoundaryReviewState, add_boundary, list_boundaries, review_prompt},
    cli::{ApprovalActionArgs, AskArgs, QueueArgs, RunArgs, approvals, core},
    paths::PraxisPaths,
    time::{Clock, SystemClock},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramCommand {
    Ask(String),
    Run(String),
    Status,
    Queue,
    Approve(i64),
    Reject(i64),
    Goal(String),
    Brief,
    Health,
    Boundaries,
    BoundariesAdd(String),
    Help,
}

pub fn parse_telegram_command(text: &str) -> TelegramCommand {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("/ask ") {
        TelegramCommand::Ask(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix("/run ") {
        TelegramCommand::Run(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix("/approve ") {
        rest.trim()
            .parse::<i64>()
            .map(TelegramCommand::Approve)
            .unwrap_or(TelegramCommand::Help)
    } else if let Some(rest) = trimmed.strip_prefix("/reject ") {
        rest.trim()
            .parse::<i64>()
            .map(TelegramCommand::Reject)
            .unwrap_or(TelegramCommand::Help)
    } else if let Some(rest) = trimmed.strip_prefix("/goal ") {
        TelegramCommand::Goal(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix("/boundaries add ") {
        TelegramCommand::BoundariesAdd(rest.trim().to_string())
    } else {
        match trimmed {
            "/status" => TelegramCommand::Status,
            "/queue" => TelegramCommand::Queue,
            "/brief" => TelegramCommand::Brief,
            "/health" => TelegramCommand::Health,
            "/boundaries" => TelegramCommand::Boundaries,
            _ => TelegramCommand::Help,
        }
    }
}

pub fn handle_telegram_command(data_dir: std::path::PathBuf, text: &str) -> Result<String> {
    match parse_telegram_command(text) {
        TelegramCommand::Ask(prompt) => core::handle_ask(
            Some(data_dir),
            AskArgs {
                files: Vec::new(),
                attachment_policy: "reject".to_string(),
                prompt: vec![prompt],
            },
        ),
        TelegramCommand::Run(task) => core::handle_run(
            Some(data_dir),
            RunArgs {
                once: true,
                force: true,
                task: Some(task),
            },
        ),
        TelegramCommand::Status => core::handle_status(Some(data_dir)),
        TelegramCommand::Queue => approvals::handle_queue(Some(data_dir), QueueArgs { all: false }),
        TelegramCommand::Approve(id) => approvals::handle_approval_action(
            Some(data_dir),
            ApprovalActionArgs { id, note: None },
            crate::storage::ApprovalStatus::Approved,
        ),
        TelegramCommand::Reject(id) => approvals::handle_approval_action(
            Some(data_dir),
            ApprovalActionArgs { id, note: None },
            crate::storage::ApprovalStatus::Rejected,
        ),
        TelegramCommand::Goal(description) => append_goal(data_dir, &description),
        TelegramCommand::Brief => render_brief(data_dir),
        TelegramCommand::Health => core::handle_doctor(Some(data_dir)),
        TelegramCommand::Boundaries => render_boundaries(data_dir),
        TelegramCommand::BoundariesAdd(rule) => append_boundary(data_dir, &rule),
        TelegramCommand::Help => Ok(help_text().to_string()),
    }
}

fn append_goal(data_dir: std::path::PathBuf, description: &str) -> Result<String> {
    if description.trim().is_empty() {
        bail!("goal description must not be empty");
    }
    let paths = PraxisPaths::for_data_dir(data_dir);
    let contents = fs::read_to_string(&paths.goals_file)?;
    let next_id = contents
        .lines()
        .filter_map(|line| line.split("G-").nth(1))
        .filter_map(|rest| rest.split(':').next())
        .filter_map(|digits| digits.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let goal_id = format!("G-{next_id:03}");
    let mut updated = contents.trim_end().to_string();
    updated.push_str(&format!("\n- [ ] {goal_id}: {description}\n"));
    fs::write(&paths.goals_file, updated)?;
    Ok(format!("goal: added\nid: {goal_id}\ntitle: {description}"))
}

fn render_brief(data_dir: std::path::PathBuf) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let journal = fs::read_to_string(&paths.journal_file)?;
    let lines = journal.lines().rev().take(8).collect::<Vec<_>>();
    Ok(format!(
        "brief:\n{}",
        lines.into_iter().rev().collect::<Vec<_>>().join("\n")
    ))
}

fn render_boundaries(data_dir: std::path::PathBuf) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let rules = list_boundaries(&paths.identity_file)?;
    let state = BoundaryReviewState::load_or_default(&paths.boundary_review_file)?;
    let mut lines = Vec::new();
    if let Some(prompt) = review_prompt(&state, SystemClock::from_env()?.now_utc()) {
        lines.push(prompt);
    }
    if rules.is_empty() {
        lines.push("boundaries: none explicitly listed yet.".to_string());
        return Ok(lines.join("\n"));
    }
    lines.push("boundaries:".to_string());
    lines.extend(rules.into_iter().map(|rule| format!("- {rule}")));
    Ok(lines.join("\n"))
}

fn append_boundary(data_dir: std::path::PathBuf, rule: &str) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    add_boundary(&paths.identity_file, rule)?;
    Ok(format!("boundary: added\nrule: {rule}"))
}

fn help_text() -> &'static str {
    "supported: /ask /run /status /queue /approve /reject /goal /brief /health /boundaries /boundaries add\n/ask is low-latency and stateless. /run executes a real Praxis session and updates durable state."
}

#[cfg(test)]
mod tests {
    use super::{TelegramCommand, parse_telegram_command};

    #[test]
    fn parses_task_commands() {
        assert_eq!(
            parse_telegram_command("/ask draft a roadmap"),
            TelegramCommand::Ask("draft a roadmap".to_string())
        );
        assert_eq!(
            parse_telegram_command("/run ship the loop"),
            TelegramCommand::Run("ship the loop".to_string())
        );
    }

    #[test]
    fn parses_action_commands() {
        assert_eq!(
            parse_telegram_command("/approve 12"),
            TelegramCommand::Approve(12)
        );
        assert_eq!(parse_telegram_command("/queue"), TelegramCommand::Queue);
        assert_eq!(parse_telegram_command("/unknown"), TelegramCommand::Help);
    }
}
