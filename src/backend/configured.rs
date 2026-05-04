use anyhow::{Result, bail};

use crate::{
    canary::RouteWeightStore,
    config::AppConfig,
    identity::Goal,
    paths::PraxisPaths,
    providers::{ProviderProtocol, ProviderRoute, ProviderSettings, RouteClass},
};

use super::{
    AgentBackend, BackendOutput, ProviderRequest, StubBackend,
    attempts::failed_attempt,
    claude,
    gating::CanaryGate,
    ollama, openai,
    prompts::{request_for_ask, request_for_finalize, request_for_plan},
    provider_routes::{default_route, route_for, validate_provider, route_for_with_override},
};

pub enum ConfiguredBackend {
    Stub(StubBackend),
    Single(Box<SingleBackend>),
    Router(RouterBackend),
}

#[derive(Debug, Clone)]
pub struct SingleBackend {
    route: ProviderRoute,
    local_route: ProviderRoute,
    local_first_fallback: bool,
    prompt_caching: bool,
    canary_gate: CanaryGate,
}

#[derive(Debug, Clone)]
pub struct RouterBackend {
    routes: Vec<ProviderRoute>,
    local_first_fallback: bool,
    prompt_caching: bool,
    canary_gate: CanaryGate,
    /// Dynamic weights from canary automation. Loaded once at construction time.
    route_weights: RouteWeightStore,
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
                prompt_caching: config.agent.prompt_caching,
                canary_gate,
                route_weights: RouteWeightStore::load_or_default(&paths.route_weights_file)
                    .unwrap_or_default(),
            }),
            provider => Self::Single(Box::new(SingleBackend {
                route: route_for_with_override(provider, config, &settings, &paths.data_dir)?,
                local_route: settings
                    .first_for("ollama")
                    .unwrap_or_else(|| default_route("ollama")),
                local_first_fallback: config.agent.local_first_fallback,
                prompt_caching: config.agent.prompt_caching,
                canary_gate,
            })),
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
        validate_provider(&route_for(config.agent.backend.as_str(), config, &settings)?)
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
                inner.prompt_caching,
            ),
            Self::Router(inner) => execute_routes(
                &inner.routable_for_class(Some(&RouteClass::Fast), "ask")?,
                request_for_ask(prompt),
                inner.prompt_caching,
            ),
        }
    }

    fn plan_action(
        &self,
        goal: Option<&Goal>,
        task: Option<&str>,
        context: Option<&str>,
    ) -> Result<BackendOutput> {
        let request = request_for_plan(goal, task, context);
        match self {
            Self::Stub(inner) => inner.plan_action(goal, task, context),
            Self::Single(inner) => {
                execute_routes(&inner.routable(&request)?, request, inner.prompt_caching)
            }
            Self::Router(inner) => execute_routes(
                &inner.routable_for_class(Some(&RouteClass::Reliable), "decide")?,
                request,
                inner.prompt_caching,
            ),
        }
    }

    fn finalize_action(
        &self,
        planned_summary: &str,
        goal: Option<&Goal>,
        task: Option<&str>,
        context: Option<&str>,
    ) -> Result<BackendOutput> {
        let request = request_for_finalize(planned_summary, goal, task, context);
        match self {
            Self::Stub(inner) => inner.finalize_action(planned_summary, goal, task, context),
            Self::Single(inner) => {
                execute_routes(&inner.routable(&request)?, request, inner.prompt_caching)
            }
            Self::Router(inner) => execute_routes(
                &inner.routable_for_class(Some(&RouteClass::Reliable), "act")?,
                request,
                inner.prompt_caching,
            ),
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
    /// Return an ordered route list for the given class preference and phase.
    /// Class selection:
    ///   1. Routes matching the requested class
    ///   2. Routes with no class assigned (unclassed = any)
    ///   3. All remaining routes as last-resort fallbacks
    ///
    /// Within each bucket, routes are sorted by effective weight (static weight × dynamic
    /// weight from canary automation) in descending order.  Routes with effective weight
    /// 0.0 are moved to the back so they act as final fallbacks rather than being dropped
    /// entirely (freeze / canary gate handles full exclusion).
    ///
    /// Local-first ordering is applied within the top bucket when the flag is set.
    fn routes_for_class(&self, class: Option<&RouteClass>, phase: &str) -> Vec<ProviderRoute> {
        let effective_weight = |route: &ProviderRoute| -> f64 {
            let static_w = route.weight.unwrap_or(1.0).clamp(0.0, 1.0);
            let dynamic_w = self.route_weights.get(&route.provider, &route.model);
            static_w * dynamic_w
        };

        let mut matched: Vec<ProviderRoute> = Vec::new();
        let mut unclassed: Vec<ProviderRoute> = Vec::new();
        let mut rest: Vec<ProviderRoute> = Vec::new();

        for route in &self.routes {
            match (&route.class, class) {
                (Some(rc), Some(rc2)) if rc == rc2 => matched.push(route.clone()),
                (None, _) => unclassed.push(route.clone()),
                _ => rest.push(route.clone()),
            }
        }

        let sort_by_weight = |routes: &mut Vec<ProviderRoute>| {
            routes.sort_by(|a, b| {
                effective_weight(b)
                    .partial_cmp(&effective_weight(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        };

        sort_by_weight(&mut matched);
        sort_by_weight(&mut unclassed);
        sort_by_weight(&mut rest);

        let mut ordered = matched;
        ordered.extend(unclassed);
        ordered.extend(rest);

        if self.local_first_fallback && matches!(phase, "ask" | "act") {
            let (local, remote): (Vec<_>, Vec<_>) =
                ordered.into_iter().partition(|r| r.provider == "ollama");
            ordered = local;
            ordered.extend(remote);
        }

        ordered
    }

    fn routable_for_class(
        &self,
        class: Option<&RouteClass>,
        phase: &str,
    ) -> Result<Vec<ProviderRoute>> {
        self.canary_gate.filter_routes(self.routes_for_class(class, phase))
    }
}

fn execute_routes(
    routes: &[ProviderRoute],
    request: ProviderRequest,
    prompt_caching: bool,
) -> Result<BackendOutput> {
    use super::health::provider_health;

    let mut attempts = Vec::new();
    let route_count = routes.len();

    // Filter out unhealthy providers first
    let healthy_routes: Vec<_> = routes
        .iter()
        .filter(|route| provider_health().is_healthy(&route.provider))
        .collect();

    let routes_to_try = if healthy_routes.is_empty() {
        // If all providers are unhealthy, try all of them anyway
        routes.iter().collect()
    } else {
        healthy_routes
    };

    for (i, route) in routes_to_try.iter().enumerate() {
        let start = std::time::Instant::now();
        match execute_single(route, &request, prompt_caching) {
            Ok(mut output) => {
                let response_ms = start.elapsed().as_millis() as u64;
                provider_health().record_success(&route.provider, response_ms);

                if i > 0 {
                    log::info!(
                        "provider fallback succeeded on route {} of {} during {}",
                        i + 1,
                        route_count,
                        request.phase
                    );
                }
                attempts.append(&mut output.attempts);
                return Ok(BackendOutput {
                    summary: output.summary,
                    attempts,
                });
            }
            Err(error) => {
                provider_health().record_failure(&route.provider, &error.to_string());

                log::warn!(
                    "provider '{}' failed during {} (route {} of {}): {error:#}",
                    route.provider,
                    request.phase,
                    i + 1,
                    route_count,
                );
                attempts.push(failed_attempt(route, request.phase, error));
            }
        }
    }
    bail!("all configured providers failed during {}", request.phase)
}

fn execute_single(
    route: &ProviderRoute,
    request: &ProviderRequest,
    prompt_caching: bool,
) -> Result<BackendOutput> {
    validate_provider(route)?;
    let response = match route.resolved_protocol() {
        ProviderProtocol::Anthropic => claude::execute(route, request, prompt_caching),
        ProviderProtocol::Ollama => ollama::execute(route, request),
        ProviderProtocol::OpenAiCompat => openai::execute(route, request),
    }?;
    Ok(BackendOutput {
        summary: response.summary,
        attempts: vec![response.attempt],
    })
}
