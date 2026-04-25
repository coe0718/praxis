//! A2A client — delegates tasks to remote agents.

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client;

use super::types::*;

pub struct A2aClient {
    client: Client,
    agent_url: String,
}

impl A2aClient {
    pub fn new(agent_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .context("failed to build A2A HTTP client")?,
            agent_url: agent_url.into(),
        })
    }

    /// Fetch the remote agent's public card.
    pub fn fetch_agent_card(&self) -> Result<AgentCard> {
        let url = format!("{}/.well-known/agent.json", self.agent_url.trim_end_matches('/'));
        self.client
            .get(&url)
            .send()
            .with_context(|| format!("failed to fetch agent card from {url}"))?
            .json::<AgentCard>()
            .with_context(|| format!("invalid agent card JSON at {url}"))
    }

    /// Send a task to the remote agent and wait for completion.
    pub fn send_task(&self, req: &SendTaskRequest) -> Result<SendTaskResponse> {
        let url = format!("{}/tasks/send", self.agent_url.trim_end_matches('/'));
        self.client
            .post(&url)
            .json(req)
            .send()
            .with_context(|| format!("failed to send task to {url}"))?
            .json::<SendTaskResponse>()
            .with_context(|| format!("invalid task response from {url}"))
    }

    /// Poll for the current status of a task.
    pub fn get_task(&self, task_id: &str) -> Result<Task> {
        let url = format!("{}/tasks/get", self.agent_url.trim_end_matches('/'));
        let req = GetTaskRequest {
            id: task_id.to_string(),
            history_length: Some(10),
        };
        self.client
            .post(&url)
            .json(&req)
            .send()
            .with_context(|| format!("failed to get task from {url}"))?
            .json::<Task>()
            .with_context(|| format!("invalid task JSON from {url}"))
    }

    /// Cancel a previously submitted task.
    pub fn cancel_task(&self, task_id: &str) -> Result<CancelTaskResponse> {
        let url = format!("{}/tasks/cancel", self.agent_url.trim_end_matches('/'));
        let req = CancelTaskRequest {
            id: task_id.to_string(),
        };
        self.client
            .post(&url)
            .json(&req)
            .send()
            .with_context(|| format!("failed to cancel task at {url}"))?
            .json::<CancelTaskResponse>()
            .with_context(|| format!("invalid cancel response from {url}"))
    }
}
