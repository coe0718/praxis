use anyhow::{Context, Result, bail};

use crate::{config::AppConfig, providers::{ProviderRoute, ProviderSettings}};

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
            input_cost_per_million_usd: None,
            output_cost_per_million_usd: None,
        })
}

pub fn validate_provider(route: &ProviderRoute) -> Result<()> {
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
