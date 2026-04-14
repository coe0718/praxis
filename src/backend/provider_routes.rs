use anyhow::{Context, Result, bail};

use crate::{
    config::AppConfig,
    providers::{ProviderProtocol, ProviderRoute, ProviderSettings},
};

pub fn route_for(
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

pub fn default_route(provider: &str) -> ProviderRoute {
    ProviderSettings::default()
        .first_for(provider)
        .unwrap_or(ProviderRoute {
            provider: provider.to_string(),
            model: "unknown".to_string(),
            base_url: None,
            protocol: None,
            class: None,
            input_cost_per_million_usd: None,
            output_cost_per_million_usd: None,
        })
}

/// Check that the required credentials are present for this route.
/// Custom provider names (OpenAI-compatible endpoints) look for
/// `<UPPER_PROVIDER>_API_KEY` then fall back to `OPENAI_API_KEY`.
pub fn validate_provider(route: &ProviderRoute) -> Result<()> {
    match route.resolved_protocol() {
        ProviderProtocol::Anthropic => {
            if std::env::var("PRAXIS_CLAUDE_STUB_RESPONSE").is_err() {
                let _ = std::env::var("ANTHROPIC_API_KEY")
                    .context("ANTHROPIC_API_KEY is required for the Claude provider")?;
            }
        }
        ProviderProtocol::OpenAiCompat => {
            let upper = route.provider.to_uppercase().replace('-', "_");
            let specific = format!("{upper}_API_KEY");
            let has_stub = std::env::var("PRAXIS_OPENAI_STUB_RESPONSE").is_ok()
                || std::env::var(format!("PRAXIS_{upper}_STUB_RESPONSE")).is_ok();
            if !has_stub
                && std::env::var(&specific).is_err()
                && std::env::var("OPENAI_API_KEY").is_err()
            {
                bail!(
                    "no API key found for provider '{}': set {specific} or OPENAI_API_KEY",
                    route.provider
                );
            }
        }
        ProviderProtocol::Ollama => {
            // Ollama is local; no key required.
        }
    }
    Ok(())
}
