use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{config::AppConfig, identity::Goal};

use super::AgentBackend;

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-latest";

pub struct ClaudeBackend {
    client: Client,
    api_key: String,
    model: String,
}

impl ClaudeBackend {
    pub fn from_config(config: &AppConfig) -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .context("ANTHROPIC_API_KEY is required when agent.backend = \"claude\"")?;
        let model = config
            .agent
            .model_pin
            .clone()
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .context("failed to build Claude HTTP client")?;
        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

impl AgentBackend for ClaudeBackend {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<String> {
        let target = render_target(goal, task)?;
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 180,
            system: "You are Praxis, a careful personal AI agent. Respond with one concise action summary describing the next safe step.",
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: format!("Plan the next step for this Praxis session:\n\n{target}"),
            }],
        };
        self.send_request(request)
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
    ) -> Result<String> {
        let target = render_target(goal, task)?;
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 180,
            system: "You are Praxis in the act phase. Write one concise operator-facing progress note. Do not claim external actions happened unless explicitly stated in the prompt.",
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: format!(
                    "Task context:\n{target}\n\nPlanned summary:\n{planned_summary}\n\nReturn a single concise status update."
                ),
            }],
        };
        self.send_request(request)
    }
}

fn render_target(goal: Option<&Goal>, task: Option<&str>) -> Result<String> {
    if let Some(task) = task {
        return Ok(format!("Operator task: {task}"));
    }
    if let Some(goal) = goal {
        return Ok(format!("Goal {}: {}", goal.id, goal.title));
    }
    bail!("Claude backend needs a goal or task to plan")
}

impl ClaudeBackend {
    fn send_request(&self, request: ClaudeRequest) -> Result<String> {
        let response = self
            .client
            .post(ANTHROPIC_URL)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("x-api-key", &self.api_key)
            .json(&request)
            .send()
            .context("failed to call Claude backend")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            bail!("Claude backend request failed with {status}: {body}");
        }

        let parsed = response
            .json::<ClaudeResponse>()
            .context("failed to parse Claude backend response")?;
        parsed
            .content
            .into_iter()
            .find_map(|block| match block {
                ClaudeContent::Text { text } => Some(text.trim().to_string()),
                ClaudeContent::Other => None,
            })
            .filter(|text| !text.is_empty())
            .context("Claude backend returned no text content")
    }
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: &'static str,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}
