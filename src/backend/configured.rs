use anyhow::{Context, Result, bail};

use crate::{
    config::AppConfig,
    identity::Goal,
    paths::PraxisPaths,
    providers::{ProviderRoute, ProviderSettings},
};

use super::{
    AgentBackend, BackendOutput, ProviderRequest, StubBackend,
    attempts::failed_attempt,
    claude, ollama, openai,
    prompts::{request_for_ask, request_for_finalize, request_for_plan},
};

pub enum ConfiguredBackend {
    Stub(StubBackend),
    Single(SingleBackend),
    Router(RouterBackend),
}

#[derive(Debug, Clone)]
pub struct SingleBackend {
    route: ProviderRoute,
    local_route: ProviderRoute,
    local_first_fallback: bool,
}

#[derive(Debug, Clone)]
pub struct RouterBackend {
    routes: Vec<ProviderRoute>,
    local_first_fallback: bool,
}

impl ConfiguredBackend {
    pub fn from_runtime(config: &AppConfig, paths: &PraxisPaths) -> Result<Self> {
        let settings = ProviderSettings::load_or_default(&paths.providers_file)?;
        Ok(match config.agent.backend.as_str() {
            "stub" => Self::Stub(StubBackend),
            "router" => Self::Router(RouterBackend {
                routes: settings.providers,
                local_first_fallback: config.agent.local_first_fallback,
            }),
            provider => Self::Single(SingleBackend {
                route: route_for(provider, config, &settings)?,
                local_route: settings
                    .first_for("ollama")
                    .unwrap_or_else(|| default_route("ollama")),
                local_first_fallback: config.agent.local_first_fallback,
            }),
        })
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
            Self::Single(inner) => match inner.route.provider.as_str() {
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
            Self::Single(inner) => execute_routes(
                &inner.routes_for(&request_for_ask(prompt)),
                request_for_ask(prompt),
            ),
            Self::Router(inner) => {
                execute_routes(&inner.routes_for("ask"), request_for_ask(prompt))
            }
        }
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<BackendOutput> {
        let request = request_for_plan(goal, task);
        match self {
            Self::Stub(inner) => inner.plan_action(goal, task),
            Self::Single(inner) => execute_routes(&inner.routes_for(&request), request),
            Self::Router(inner) => execute_routes(&inner.routes_for("decide"), request),
        }
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
    ) -> Result<BackendOutput> {
        let request = request_for_finalize(planned_summary, goal, task);
        match self {
            Self::Stub(inner) => inner.finalize_action(planned_summary, goal, task),
            Self::Single(inner) => execute_routes(&inner.routes_for(&request), request),
            Self::Router(inner) => execute_routes(&inner.routes_for("act"), request),
        }
    }
}

impl SingleBackend {
    fn routes_for(&self, request: &ProviderRequest) -> Vec<ProviderRoute> {
        if self.local_first_fallback
            && matches!(request.phase, "ask" | "act")
            && self.route.provider != "ollama"
        {
            return vec![self.local_route.clone(), self.route.clone()];
        }
        vec![self.route.clone()]
    }
}

impl RouterBackend {
    fn routes_for(&self, phase: &str) -> Vec<ProviderRoute> {
        if !self.local_first_fallback || !matches!(phase, "ask" | "act") {
            return self.routes.clone();
        }
        let mut local = self
            .routes
            .iter()
            .filter(|route| route.provider == "ollama")
            .cloned()
            .collect::<Vec<_>>();
        local.extend(
            self.routes
                .iter()
                .filter(|route| route.provider != "ollama")
                .cloned(),
        );
        local
    }
}

fn execute_routes(routes: &[ProviderRoute], request: ProviderRequest) -> Result<BackendOutput> {
    let mut attempts = Vec::new();
    for route in routes {
        match execute_single(route, &request) {
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

fn execute_single(route: &ProviderRoute, request: &ProviderRequest) -> Result<BackendOutput> {
    validate_provider(route)?;
    let response = match route.provider.as_str() {
        "claude" => claude::execute(route, request),
        "openai" => openai::execute(route, request),
        "ollama" => ollama::execute(route, request),
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
