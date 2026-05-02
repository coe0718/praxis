//! Interactive session REPL with slash commands.
//!
//! Provides an interactive CLI mode (`praxis chat`) where the operator can
//! converse with the agent, use slash commands, and manage sessions in real-time.

use std::{
    collections::HashMap,
    io::{self, Write},
    path::PathBuf,
};

use anyhow::{Context, Result, bail};

use crate::paths::{PraxisPaths, default_data_dir};

/// Slash command handler signature.
type CommandFn = fn(&mut ReplContext, &[&str]) -> Result<String>;

/// Context available to all slash command handlers.
pub struct ReplContext {
    pub paths: PraxisPaths,
    pub verbose: bool,
    pub model: Option<String>,
    pub history: Vec<String>,
}

/// A registered slash command.
struct SlashCommand {
    name: &'static str,
    help: &'static str,
    handler: CommandFn,
}

/// Registry of all available slash commands.
static COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "help",
        help: "Show available slash commands",
        handler: cmd_help,
    },
    SlashCommand {
        name: "new",
        help: "Start a new conversation (clear context)",
        handler: cmd_new,
    },
    SlashCommand {
        name: "model",
        help: "Show or switch the active model (/model <name>)",
        handler: cmd_model,
    },
    SlashCommand {
        name: "verbose",
        help: "Toggle verbose mode (show thinking/reasoning)",
        handler: cmd_verbose,
    },
    SlashCommand {
        name: "compact",
        help: "Compact the current context to reduce token usage",
        handler: cmd_compact,
    },
    SlashCommand {
        name: "retry",
        help: "Retry the last LLM call",
        handler: cmd_retry,
    },
    SlashCommand {
        name: "undo",
        help: "Remove the last exchange from conversation history",
        handler: cmd_undo,
    },
    SlashCommand {
        name: "history",
        help: "Show conversation history summary",
        handler: cmd_history,
    },
    SlashCommand {
        name: "skills",
        help: "List loaded skills",
        handler: cmd_skills,
    },
    SlashCommand {
        name: "tools",
        help: "List available tools",
        handler: cmd_tools,
    },
    SlashCommand {
        name: "status",
        help: "Show agent status (session count, memory, provider)",
        handler: cmd_status,
    },
    SlashCommand {
        name: "save",
        help: "Save conversation to a file (/save [path])",
        handler: cmd_save,
    },
    SlashCommand {
        name: "export",
        help: "Export conversation as markdown",
        handler: cmd_export,
    },
    SlashCommand {
        name: "quit",
        help: "Exit the interactive session (also: /exit, Ctrl+D)",
        handler: cmd_quit,
    },
    SlashCommand {
        name: "exit",
        help: "Alias for /quit",
        handler: cmd_quit,
    },
];

/// Run the interactive REPL loop.
pub fn run_interactive(data_dir_override: Option<PathBuf>, model: Option<String>) -> Result<()> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);

    if !paths.config_file.exists() {
        bail!("Praxis is not initialized. Run `praxis init` first.");
    }

    let mut ctx = ReplContext {
        paths,
        verbose: false,
        model,
        history: Vec::new(),
    };

    println!("Praxis interactive session. Type /help for commands, Ctrl+D to exit.");
    println!();

    let stdin = io::stdin();
    loop {
        print!("praxis> ");
        io::stdout().flush()?;

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => {
                // EOF (Ctrl+D)
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading input: {e}");
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') {
            let parts: Vec<&str> = input[1..].splitn(2, ' ').collect();
            let cmd_name = parts[0];
            let args: Vec<&str> = parts
                .get(1)
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();

            match handle_command(&mut ctx, cmd_name, &args) {
                Ok(output) => {
                    if output == "__QUIT__" {
                        break;
                    }
                    println!("{output}");
                }
                Err(e) => eprintln!("Error: {e:#}"),
            }
        } else {
            // Regular message — send to agent
            ctx.history.push(format!("user: {input}"));

            // For now, delegate to `praxis ask` — in a full implementation
            // this would maintain an open session with the LLM.
            match dispatch_message(&ctx, input) {
                Ok(response) => {
                    ctx.history.push(format!("assistant: {response}"));
                    println!("{response}");
                }
                Err(e) => eprintln!("Error: {e:#}"),
            }
        }
        println!();
    }

    Ok(())
}

fn handle_command(ctx: &mut ReplContext, name: &str, args: &[&str]) -> Result<String> {
    let cmd = COMMANDS
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow::anyhow!("unknown command: /{name}. Type /help for available commands."))?;

    (cmd.handler)(ctx, args)
}

/// Dispatch a user message to the agent backend.
fn dispatch_message(ctx: &ReplContext, input: &str) -> Result<String> {
    use crate::cli::core;

    let ask_args = crate::cli::AskArgs {
        files: Vec::new(),
        attachment_policy: "reject".to_string(),
        prompt: input.split_whitespace().map(String::from).collect(),
    };

    core::handle_ask(Some(ctx.paths.data_dir.clone()), ask_args)
}

// ── Slash command implementations ──

fn cmd_help(_ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    let mut lines = vec!["Available commands:".to_string()];
    let mut cmd_map: HashMap<&str, &str> = HashMap::new();
    for cmd in COMMANDS {
        cmd_map.insert(cmd.name, cmd.help);
    }
    // Deduplicate aliases (exit → quit)
    let mut seen = std::collections::HashSet::new();
    for cmd in COMMANDS {
        if seen.contains(cmd.name) {
            continue;
        }
        seen.insert(cmd.name);
        lines.push(format!("  /{:<12} {}", cmd.name, cmd.help));
    }
    lines.push(String::new());
    lines.push("Type a message without / to chat with the agent.".to_string());
    Ok(lines.join("\n"))
}

fn cmd_new(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    let count = ctx.history.len();
    ctx.history.clear();
    Ok(format!("New session started (cleared {count} history entries)."))
}

fn cmd_model(ctx: &mut ReplContext, args: &[&str]) -> Result<String> {
    match args.first() {
        Some(name) => {
            ctx.model = Some(name.to_string());
            Ok(format!("Model set to: {name}"))
        }
        None => {
            let current = ctx.model.as_deref().unwrap_or("(default from config)");
            Ok(format!("Current model: {current}"))
        }
    }
}

fn cmd_verbose(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    ctx.verbose = !ctx.verbose;
    Ok(format!("Verbose mode: {}", if ctx.verbose { "ON" } else { "OFF" }))
}

fn cmd_compact(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    use crate::context::compaction::{CompactionRequest, request_compact};
    use crate::time::{Clock, SystemClock};

    let clock = SystemClock::from_env()?;
    let now = clock.now_utc();
    let req = CompactionRequest::operator(None, now);
    request_compact(&ctx.paths.data_dir, &req)?;
    Ok("Context compaction requested.".to_string())
}

fn cmd_retry(_ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    Ok("Retry: replaying last exchange. (full implementation requires active LLM session)".to_string())
}

fn cmd_undo(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    // Remove last user + assistant pair
    if ctx.history.len() >= 2 {
        ctx.history.pop();
        ctx.history.pop();
        Ok("Last exchange removed from history.".to_string())
    } else if ctx.history.len() == 1 {
        ctx.history.pop();
        Ok("Last message removed from history.".to_string())
    } else {
        Ok("Nothing to undo.".to_string())
    }
}

fn cmd_history(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    if ctx.history.is_empty() {
        return Ok("No conversation history.".to_string());
    }
    let mut lines = vec![format!("History ({} entries):", ctx.history.len())];
    for (i, entry) in ctx.history.iter().enumerate() {
        let truncated: String = entry.chars().take(100).collect();
        let suffix = if entry.len() > 100 { "..." } else { "" };
        lines.push(format!("  [{}] {}{}", i + 1, truncated, suffix));
    }
    Ok(lines.join("\n"))
}

fn cmd_skills(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    let skills_dir = &ctx.paths.skills_dir;
    if !skills_dir.exists() {
        return Ok("No skills directory found.".to_string());
    }

    let mut skills: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(skills_dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".md") {
                skills.push(name.trim_end_matches(".md").to_string());
            }
        }
    }

    if skills.is_empty() {
        return Ok("No skills installed.".to_string());
    }

    let mut lines = vec![format!("Skills ({}):", skills.len())];
    for skill in &skills {
        lines.push(format!("  - {skill}"));
    }
    Ok(lines.join("\n"))
}

fn cmd_tools(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    let tools_dir = &ctx.paths.tools_dir;
    if !tools_dir.exists() {
        return Ok("No tools directory found.".to_string());
    }

    let mut tools: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(tools_dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            if name.ends_with(".toml") {
                tools.push(name.trim_end_matches(".toml").to_string());
            }
        }
    }

    if tools.is_empty() {
        return Ok("No tools installed.".to_string());
    }

    let mut lines = vec![format!("Tools ({}):", tools.len())];
    for tool in &tools {
        lines.push(format!("  - {tool}"));
    }
    Ok(lines.join("\n"))
}

fn cmd_status(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    use crate::storage::SqliteSessionStore;

    let store = SqliteSessionStore::new(ctx.paths.database_file.clone());
    let hot = store.count_hot_memories().unwrap_or(0);
    let cold = store.count_cold_memories().unwrap_or(0);
    let pending = store.count_pending_approvals().unwrap_or(0);

    let mut lines = vec!["Status:".to_string()];
    lines.push(format!("  history entries: {}", ctx.history.len()));
    lines.push(format!("  model: {}", ctx.model.as_deref().unwrap_or("(default)")));
    lines.push(format!("  verbose: {}", ctx.verbose));
    lines.push(format!("  memory: {hot} hot / {cold} cold"));
    lines.push(format!("  pending approvals: {pending}"));
    Ok(lines.join("\n"))
}

fn cmd_save(ctx: &mut ReplContext, args: &[&str]) -> Result<String> {
    let path = args
        .first()
        .map(|p| PathBuf::from(p))
        .unwrap_or_else(|| PathBuf::from("praxis-session.txt"));

    let content = ctx.history.join("\n\n");
    std::fs::write(&path, &content).with_context(|| format!("failed to save to {}", path.display()))?;
    Ok(format!("Session saved to {}", path.display()))
}

fn cmd_export(ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    let mut lines = vec!["# Praxis Session Export".to_string(), String::new()];
    for entry in &ctx.history {
        if let Some((role, content)) = entry.split_once(": ") {
            lines.push(format!("**{}**: {}", role.to_uppercase(), content));
        } else {
            lines.push(entry.clone());
        }
        lines.push(String::new());
    }
    let path = PathBuf::from("praxis-session.md");
    std::fs::write(&path, lines.join("\n"))?;
    Ok(format!("Session exported to {}", path.display()))
}

fn cmd_quit(_ctx: &mut ReplContext, _args: &[&str]) -> Result<String> {
    Ok("__QUIT__".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_lists_all_commands() {
        let ctx = ReplContext {
            paths: PraxisPaths::for_data_dir(PathBuf::from("/tmp/test-praxis-chat")),
            verbose: false,
            model: None,
            history: Vec::new(),
        };
        let result = cmd_help(&ctx, &[]).unwrap();
        assert!(result.contains("/help"));
        assert!(result.contains("/quit"));
        assert!(result.contains("/model"));
    }

    #[test]
    fn test_model_toggle() {
        let mut ctx = ReplContext {
            paths: PraxisPaths::for_data_dir(PathBuf::from("/tmp/test-praxis-chat")),
            verbose: false,
            model: None,
            history: Vec::new(),
        };
        let result = cmd_model(&mut ctx, &["gpt-4o"]).unwrap();
        assert!(result.contains("gpt-4o"));
        assert_eq!(ctx.model, Some("gpt-4o".to_string()));

        let result = cmd_model(&mut ctx, &[]).unwrap();
        assert!(result.contains("gpt-4o"));
    }

    #[test]
    fn test_undo() {
        let mut ctx = ReplContext {
            paths: PraxisPaths::for_data_dir(PathBuf::from("/tmp/test-praxis-chat")),
            verbose: false,
            model: None,
            history: vec![
                "user: hello".to_string(),
                "assistant: hi there".to_string(),
            ],
        };
        cmd_undo(&mut ctx, &[]).unwrap();
        assert!(ctx.history.is_empty());
    }
}
