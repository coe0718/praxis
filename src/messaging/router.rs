use std::fs;

use anyhow::{Context, Result, bail};

use crate::{
    boundaries::{BoundaryReviewState, add_boundary, list_boundaries, review_prompt},
    cli::{ApprovalActionArgs, AskArgs, QueueArgs, RunArgs, approvals, core},
    memory::{MemoryLinkStore, MemoryLinkType, MemoryStore},
    messaging::{ActivationMode, ActivationStore},
    paths::PraxisPaths,
    storage::{DecisionReceiptStore, SqliteSessionStore},
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
    /// Set the activation mode for the current conversation.
    Activation(String),
    /// List recent memories with their IDs.
    Memories,
    /// Boost the importance of a memory.
    Reinforce(i64),
    /// Delete a memory.
    Forget(i64),
    /// Add a relational link between two memories.
    Link(i64, String, i64),
    /// Show the last N decision receipts.
    Decisions,
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
    } else if let Some(rest) = trimmed.strip_prefix("/activation ") {
        TelegramCommand::Activation(rest.trim().to_string())
    } else if let Some(rest) = trimmed.strip_prefix("/reinforce ") {
        rest.trim()
            .parse::<i64>()
            .map(TelegramCommand::Reinforce)
            .unwrap_or(TelegramCommand::Help)
    } else if let Some(rest) = trimmed.strip_prefix("/forget ") {
        rest.trim()
            .parse::<i64>()
            .map(TelegramCommand::Forget)
            .unwrap_or(TelegramCommand::Help)
    } else if let Some(rest) = trimmed.strip_prefix("/link ") {
        parse_link_command(rest.trim())
    } else {
        match trimmed {
            "/status" => TelegramCommand::Status,
            "/queue" => TelegramCommand::Queue,
            "/brief" => TelegramCommand::Brief,
            "/health" => TelegramCommand::Health,
            "/boundaries" => TelegramCommand::Boundaries,
            "/activation" => TelegramCommand::Activation(String::new()),
            "/memories" => TelegramCommand::Memories,
            "/decisions" => TelegramCommand::Decisions,
            _ => TelegramCommand::Help,
        }
    }
}

/// Handle a command received from a Telegram message.
///
/// `chat_id` is the Telegram chat ID of the conversation — required so that
/// activation mode changes can be scoped to the correct conversation.
pub fn handle_telegram_command(
    data_dir: std::path::PathBuf,
    chat_id: i64,
    text: &str,
) -> Result<String> {
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
        TelegramCommand::Activation(mode_str) => handle_activation(data_dir, chat_id, &mode_str),
        TelegramCommand::Memories => handle_memories(data_dir),
        TelegramCommand::Reinforce(id) => handle_reinforce(data_dir, id),
        TelegramCommand::Forget(id) => handle_forget(data_dir, id),
        TelegramCommand::Link(from_id, link_type, to_id) => {
            handle_link(data_dir, from_id, &link_type, to_id)
        }
        TelegramCommand::Decisions => handle_decisions(data_dir),
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

fn handle_activation(data_dir: std::path::PathBuf, chat_id: i64, mode_str: &str) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let mut store = ActivationStore::load(&paths.activation_file)?;

    if mode_str.is_empty() {
        let current = store.get(&chat_id.to_string());
        return Ok(format!("activation: {current}"));
    }

    let mode = parse_activation_mode(mode_str)
        .with_context(|| format!("unknown activation mode: {mode_str}"))?;
    store.set(chat_id.to_string(), mode);
    store.save(&paths.activation_file)?;
    Ok(format!("activation: set to {mode}"))
}

fn parse_activation_mode(s: &str) -> Result<ActivationMode> {
    match s {
        "mention_only" | "mention" => Ok(ActivationMode::MentionOnly),
        "thread_only" | "thread" => Ok(ActivationMode::ThreadOnly),
        "always_listening" | "always" => Ok(ActivationMode::AlwaysListening),
        _ => bail!("valid modes: mention_only, thread_only, always_listening"),
    }
}

fn parse_link_command(rest: &str) -> TelegramCommand {
    let parts: Vec<&str> = rest.splitn(3, ' ').collect();
    if parts.len() != 3 {
        return TelegramCommand::Help;
    }
    let from_id = match parts[0].parse::<i64>() {
        Ok(id) => id,
        Err(_) => return TelegramCommand::Help,
    };
    let to_id = match parts[2].parse::<i64>() {
        Ok(id) => id,
        Err(_) => return TelegramCommand::Help,
    };
    TelegramCommand::Link(from_id, parts[1].to_string(), to_id)
}

fn handle_memories(data_dir: std::path::PathBuf) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let hot = store.recent_hot_memories(5)?;
    let cold = store.strongest_cold_memories(5)?;

    if hot.is_empty() && cold.is_empty() {
        return Ok("memories: none stored yet".to_string());
    }

    let mut lines = vec!["memories:".to_string()];
    if !hot.is_empty() {
        lines.push("hot:".to_string());
        for m in &hot {
            let summary = m.summary.as_deref().unwrap_or(m.content.as_str());
            let truncated = if summary.len() > 80 {
                format!("{}…", &summary[..80])
            } else {
                summary.to_string()
            };
            lines.push(format!("  [{}] {}", m.id, truncated));
        }
    }
    if !cold.is_empty() {
        lines.push("cold:".to_string());
        for m in &cold {
            let summary = m.summary.as_deref().unwrap_or(m.content.as_str());
            let truncated = if summary.len() > 80 {
                format!("{}…", &summary[..80])
            } else {
                summary.to_string()
            };
            lines.push(format!("  [{}] {}", m.id, truncated));
        }
    }
    Ok(lines.join("\n"))
}

fn handle_reinforce(data_dir: std::path::PathBuf, id: i64) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.boost_memory(id)?;
    Ok(format!("memory: reinforced [{id}]"))
}

fn handle_forget(data_dir: std::path::PathBuf, id: i64) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    store.forget_memory(id)?;
    Ok(format!("memory: forgotten [{id}]"))
}

fn handle_link(
    data_dir: std::path::PathBuf,
    from_id: i64,
    link_type_str: &str,
    to_id: i64,
) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let link_type = MemoryLinkType::parse(link_type_str)
        .with_context(|| format!("unknown link type: {link_type_str}"))?;
    store.add_memory_link(from_id, to_id, link_type)?;
    Ok(format!(
        "memory: linked [{from_id}] --{link_type_str}--> [{to_id}]"
    ))
}

fn handle_decisions(data_dir: std::path::PathBuf) -> Result<String> {
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = SqliteSessionStore::new(paths.database_file.clone());
    let receipts = store.recent_decisions(8)?;

    if receipts.is_empty() {
        return Ok("decisions: none recorded yet".to_string());
    }

    let mut lines = vec!["decisions (oldest first):".to_string()];
    for r in &receipts {
        let goal = r
            .goal_id
            .as_deref()
            .map(|id| format!(" [{id}]"))
            .unwrap_or_default();
        let approval = if r.approval_required {
            " ⚠ approval"
        } else {
            ""
        };
        let action = if r.chosen_action.len() > 80 {
            format!("{}…", &r.chosen_action[..80])
        } else {
            r.chosen_action.clone()
        };
        lines.push(format!(
            "{:.0}%{}{}{} — {}",
            r.confidence * 100.0,
            goal,
            approval,
            format!(" ({})", r.reason_code),
            action
        ));
    }
    Ok(lines.join("\n"))
}

fn help_text() -> &'static str {
    "supported commands:\n\
     /ask <prompt> — quick stateless question\n\
     /run <task> — full Praxis session\n\
     /status — session and goal status\n\
     /queue — list pending tool approvals\n\
     /approve <id> — approve a queued tool request\n\
     /reject <id> — reject a queued tool request\n\
     /goal <description> — add a new goal\n\
     /brief — last 8 journal lines\n\
     /health — system health check\n\
     /boundaries — list active boundaries\n\
     /boundaries add <rule> — add a boundary\n\
     /activation [mention_only|thread_only|always_listening] — get or set activation mode\n\
     /memories — list recent memories with IDs\n\
     /reinforce <id> — boost memory importance\n\
     /forget <id> — delete a memory\n\
     /link <from_id> <type> <to_id> — add relational link (types: caused_by, related_to, contradicts, user_preference, follow_up)\n\
     /decisions — last 8 decide-phase receipts"
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

    #[test]
    fn parses_activation_commands() {
        assert_eq!(
            parse_telegram_command("/activation always_listening"),
            TelegramCommand::Activation("always_listening".to_string())
        );
        assert_eq!(
            parse_telegram_command("/activation"),
            TelegramCommand::Activation(String::new())
        );
    }
}
