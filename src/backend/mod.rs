mod attempts;
mod claude;
mod configured;
mod credential_pool;
mod gating;
mod ollama;
mod openai;
mod prompts;
mod provider_routes;
pub mod streaming;

pub use credential_pool::{CredentialPool, init_pools};

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

#[derive(Debug, Clone)]
pub struct ProviderRequest {
    pub phase: &'static str,
    pub system: String,
    pub input: String,
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
