mod attempts;
mod claude;
mod ollama;
mod openai;
mod prompts;

use anyhow::{Context, Result, bail};

use crate::{
    config::AppConfig,
    identity::Goal,
    paths::PraxisPaths,
    providers::{ProviderRoute, ProviderSettings},
    usage::ProviderAttempt,
};

use self::{
    attempts::{failed_attempt, successful_attempt},
    prompts::{
        render_stub_answer, render_stub_summary, request_for_ask, request_for_finalize,
        request_for_plan,
    },
};

pub trait AgentBackend {
    fn name(&self) -> &'static str;
    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput>;
    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<BackendOutput>;
    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
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
    pub system: &'static str,
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

pub enum ConfiguredBackend {
    Stub(StubBackend),
    Single(ProviderRoute),
    Router(Vec<ProviderRoute>),
}

impl ConfiguredBackend {
    pub fn from_runtime(config: &AppConfig, paths: &PraxisPaths) -> Result<Self> {
        let settings = ProviderSettings::load_or_default(&paths.providers_file)?;
        let backend = match config.agent.backend.as_str() {
            "stub" => Self::Stub(StubBackend),
            "router" => Self::Router(settings.providers),
            provider => Self::Single(route_for(provider, config, &settings)?),
        };
        Ok(backend)
    }

    pub fn validate_environment(config: &AppConfig, paths: &PraxisPaths) -> Result<()> {
        if config.agent.backend == "stub" {
            return Ok(());
        }

        let settings = ProviderSettings::load_or_default(&paths.providers_file)?;
        if config.agent.backend == "router" {
            return settings.validate();
        }

        validate_provider(&route_for(
            config.agent.backend.as_str(),
            config,
            &settings,
        )?)
    }
}

impl AgentBackend for ConfiguredBackend {
    fn name(&self) -> &'static str {
        match self {
            Self::Stub(inner) => inner.name(),
            Self::Single(route) => match route.provider.as_str() {
                "claude" => "claude",
                "openai" => "openai",
                "ollama" => "ollama",
                _ => "provider",
            },
            Self::Router(_) => "router",
        }
    }

    fn answer_prompt(&self, prompt: &str) -> Result<BackendOutput> {
        match self {
            Self::Stub(inner) => inner.answer_prompt(prompt),
            Self::Single(route) => execute_single(route, request_for_ask(prompt)),
            Self::Router(routes) => execute_router(routes, request_for_ask(prompt)),
        }
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<BackendOutput> {
        match self {
            Self::Stub(inner) => inner.plan_action(goal, task),
            Self::Single(route) => execute_single(route, request_for_plan(goal, task)),
            Self::Router(routes) => execute_router(routes, request_for_plan(goal, task)),
        }
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
    ) -> Result<BackendOutput> {
        match self {
            Self::Stub(inner) => inner.finalize_action(planned_summary, goal, task),
            Self::Single(route) => {
                execute_single(route, request_for_finalize(planned_summary, goal, task))
            }
            Self::Router(routes) => {
                execute_router(routes, request_for_finalize(planned_summary, goal, task))
            }
        }
    }
}

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

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<BackendOutput> {
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
    ) -> Result<BackendOutput> {
        Ok(BackendOutput {
            summary: format!(
                "{planned_summary} Act phase completed without external side effects."
            ),
            attempts: vec![successful_attempt("act", "stub", "stub", 0, 0, 0)],
        })
    }
}

fn execute_router(routes: &[ProviderRoute], request: ProviderRequest) -> Result<BackendOutput> {
    let mut attempts = Vec::new();
    for route in routes {
        match execute_single(route, request.clone()) {
            Ok(mut output) => {
                attempts.append(&mut output.attempts);
                return Ok(BackendOutput {
                    summary: output.summary,
                    attempts,
                });
            }
            Err(error) => attempts.push(failed_attempt(route, request.phase, error)),
        }
    }

    bail!("all configured providers failed during {}", request.phase)
}

fn execute_single(route: &ProviderRoute, request: ProviderRequest) -> Result<BackendOutput> {
    validate_provider(route)?;
    let response = match route.provider.as_str() {
        "claude" => claude::execute(route, &request),
        "openai" => openai::execute(route, &request),
        "ollama" => ollama::execute(route, &request),
        other => bail!("unsupported provider {other}"),
    }?;

    Ok(BackendOutput {
        summary: response.summary,
        attempts: vec![response.attempt],
    })
}

fn route_for(
    provider: &str,
    config: &AppConfig,
    settings: &ProviderSettings,
) -> Result<ProviderRoute> {
    let mut route = settings
        .first_for(provider)
        .unwrap_or_else(|| default_route(provider));
    if config.agent.model_pin.is_some() && provider == config.agent.backend {
        route.model = config.agent.model_pin.clone().unwrap_or(route.model);
    }
    route.validate()?;
    Ok(route)
}

fn default_route(provider: &str) -> ProviderRoute {
    ProviderSettings::default()
        .first_for(provider)
        .unwrap_or(ProviderRoute {
            provider: provider.to_string(),
            model: "unknown".to_string(),
            base_url: None,
            input_cost_per_million_usd: None,
            output_cost_per_million_usd: None,
        })
}

fn validate_provider(route: &ProviderRoute) -> Result<()> {
    match route.provider.as_str() {
        "claude" if std::env::var("PRAXIS_CLAUDE_STUB_RESPONSE").is_err() => {
            let _ = std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY is required for the Claude provider")?;
        }
        "openai" if std::env::var("PRAXIS_OPENAI_STUB_RESPONSE").is_err() => {
            let _ = std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY is required for the OpenAI provider")?;
        }
        "ollama" | "claude" | "openai" => {}
        other => bail!("unsupported provider {other}"),
    }
    Ok(())
}
