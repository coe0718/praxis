use anyhow::{Result, bail};

use crate::{
    canary::ModelCanaryLedger, config::AppConfig, paths::PraxisPaths, providers::ProviderRoute,
};

#[derive(Debug, Clone)]
pub struct CanaryGate {
    enabled: bool,
    ledger: ModelCanaryLedger,
}

impl CanaryGate {
    pub fn from_runtime(config: &AppConfig, paths: &PraxisPaths) -> Result<Self> {
        Ok(Self {
            enabled: config.agent.freeze_on_model_regression,
            ledger: ModelCanaryLedger::load_or_default(&paths.model_canary_file)?,
        })
    }

    pub fn filter_routes(&self, routes: Vec<ProviderRoute>) -> Result<Vec<ProviderRoute>> {
        if !self.enabled {
            return Ok(routes);
        }

        let allowed = routes
            .into_iter()
            .filter(|route| self.route_allowed(route))
            .collect::<Vec<_>>();
        if !allowed.is_empty() {
            return Ok(allowed);
        }

        bail!(
            "all configured remote provider models are frozen until a passing canary is recorded; run `praxis canary run`"
        )
    }

    fn route_allowed(&self, route: &ProviderRoute) -> bool {
        route.provider == "ollama" || self.ledger.passed(&route.provider, &route.model)
    }
}
