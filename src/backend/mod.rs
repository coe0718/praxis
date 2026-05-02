mod attempts;
mod claude;
mod configured;
mod credential_pool;
mod gating;
mod health;
mod ollama;
mod openai;
mod prompts;
mod provider_routes;
pub mod streaming;

pub(crate) use credential_pool::init_pools;
/// Re-exported for cross-module tool use (e.g., image generation).
pub(crate) use openai::resolve_api_key;

use anyhow::Result;

use crate::{identity::Goal, usage::ProviderAttempt};

pub use configured::ConfiguredBackend;

use self::{
    attempts::successful_attempt,
    prompts::{render_stub_answer, render_stub_summary},
};

pub trait AgentBackend {
    fn name(&self) -> &'static str;
    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput>;
    fn plan_action(
        &self,
        goal: Option<&Goal>,
        task: Option<&str>,
        context: Option<&str>,
    ) -> Result<BackendOutput>;
    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
        context: Option<&str>,
    ) -> Result<BackendOutput>;
}

#[derive(Debug, Clone)]
pub struct BackendOutput {
    pub summary: String,
    pub attempts: Vec<ProviderAttempt>,
}

/// Multi-modal content block for vision support.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: ImageUrl,
    },
}

/// Image URL with optional detail level.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Input content that can be either a simple string or multi-modal blocks.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum InputContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub phase: &'static str,
    pub system: String,
    pub input: InputContent,
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub summary: String,
    pub attempt: ProviderAttempt,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StubBackend;

impl AgentBackend for StubBackend {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: render_stub_answer(prompt),
            attempts: vec![successful_attempt("ask", "stub", "stub", 0, 0, 0)],
        })
    }

    fn plan_action(
        &self,
        goal: Option<&Goal>,
        task: Option<&str>,
        _context: Option<&str>,
    ) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: render_stub_summary(goal, task),
            attempts: vec![successful_attempt("decide", "stub", "stub", 0, 0, 0)],
        })
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        _goal: Option<&Goal>,
        _task: Option<&str>,
        _context: Option<&str>,
    ) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: format!(
                "{planned_summary} Act phase completed without external side effects."
            ),
            attempts: vec![successful_attempt("act", "stub", "stub", 0, 0, 0)],
        })
    }
}
