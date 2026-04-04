use anyhow::{Result, bail};

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
    gating::CanaryGate,
    prompts::{request_for_ask, request_for_finalize, request_for_plan},
    provider_routes::{default_route, route_for, validate_provider},
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
    canary_gate: CanaryGate,
}

#[derive(Debug, Clone)]
pub struct RouterBackend {
    routes: Vec<ProviderRoute>,
    local_first_fallback: bool,
    canary_gate: CanaryGate,
}

impl ConfiguredBackend {
    pub fn from_runtime(config: &AppConfig, paths: &PraxisPaths) -> Result<Self> {
        let settings = ProviderSettings::load_or_default(&paths.providers_file)?;
        let canary_gate = CanaryGate::from_runtime(config, paths)?;
        Ok(match config.agent.backend.as_str() {
            "stub" => Self::Stub(StubBackend),
            "router" => Self::Router(RouterBackend {
                routes: settings.providers,
                local_first_fallback: config.agent.local_first_fallback,
                canary_gate,
            }),
            provider => Self::Single(SingleBackend {
                route: route_for(provider, config, &settings)?,
                local_route: settings
                    .first_for("ollama")
                    .unwrap_or_else(|| default_route("ollama")),
                local_first_fallback: config.agent.local_first_fallback,
                canary_gate,
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
                &inner.routable(&request_for_ask(prompt))?,
                request_for_ask(prompt),
            ),
            Self::Router(inner) => {
                execute_routes(&inner.routable("ask")?, request_for_ask(prompt))
            }
        }
    }

    fn plan_action(&self, goal: Option<&Goal>, task: Option<&str>) -> Result<BackendOutput> {
        let request = request_for_plan(goal, task);
        match self {
            Self::Stub(inner) => inner.plan_action(goal, task),
            Self::Single(inner) => execute_routes(&inner.routable(&request)?, request),
            Self::Router(inner) => execute_routes(&inner.routable("decide")?, request),
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
            Self::Single(inner) => execute_routes(&inner.routable(&request)?, request),
            Self::Router(inner) => execute_routes(&inner.routable("act")?, request),
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

    fn routable(&self, request: &ProviderRequest) -> Result<Vec<ProviderRoute>> {
        self.canary_gate.filter_routes(self.routes_for(request))
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

    fn routable(&self, phase: &str) -> Result<Vec<ProviderRoute>> {
        self.canary_gate.filter_routes(self.routes_for(phase))
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
