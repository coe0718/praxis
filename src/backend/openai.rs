use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::{ProviderRequest, ProviderResponse, successful_attempt};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// Execute a request against any OpenAI-compatible Chat Completions endpoint.
/// Works with OpenAI, Together, Groq, Mistral, local llama.cpp, and any other
/// provider that exposes `/v1/chat/completions`.
///
/// Auth is `Authorization: Bearer <key>`.  The key is read from:
///   - `OPENAI_API_KEY` for the "openai" provider
///   - `<UPPER_PROVIDER>_API_KEY` for custom provider names (e.g. `GROQ_API_KEY`)
///   - `OPENAI_API_KEY` as the final fallback for custom providers
pub(super) fn execute(
    route: &ProviderRoute,
    request: &ProviderRequest,
) -> Result<ProviderResponse> {
    if let Some(response) = stubbed(route, request.phase)? {
        return Ok(response);
    }

    let api_key = resolve_api_key(&route.provider)?;
    let endpoint = resolve_endpoint(route);

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build HTTP client for OpenAI-compatible provider")?;

    let body = ChatRequest {
        model: route.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: request.system.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: request.input.clone(),
            },
        ],
        max_completion_tokens: Some(request.max_output_tokens),
    };

    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .with_context(|| format!("failed to call provider {}", route.provider))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        let safe_body = body.chars().take(200).collect::<String>();
        bail!("provider {} request failed with {status}: {safe_body}", route.provider);
    }

    let parsed = response
        .json::<ChatResponse>()
        .with_context(|| format!("failed to parse response from provider {}", route.provider))?;

    let text = parsed
        .choices
        .into_iter()
        .find_map(|choice| {
            let content = choice.message.content.trim().to_string();
            (!content.is_empty()).then_some(content)
        })
        .with_context(|| format!("provider {} returned no text content", route.provider))?;

    let input_tokens = parsed.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
    let output_tokens = parsed.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);

    Ok(ProviderResponse {
        summary: text,
        attempt: successful_attempt(
            request.phase,
            &route.provider,
            &route.model,
            input_tokens,
            output_tokens,
            route.estimated_cost_micros(input_tokens, output_tokens),
        ),
    })
}

pub(crate) fn resolve_api_key(provider: &str) -> Result<String> {
    // If credential pooling is enabled and a pool exists, rotate through it.
    if let Some(pool) = super::credential_pool::CREDENTIAL_POOLS
        .get()
        .and_then(|pools| pools.get(provider))
    {
        return Ok(pool.next_key().to_string());
    }

    // 1. Try <UPPER_PROVIDER>_API_KEY env var.
    let env_name = format!("{}_API_KEY", provider.to_uppercase().replace('-', "_"));
    if let Ok(key) = std::env::var(&env_name) {
        return Ok(key);
    }
    // 2. Try OPENAI_API_KEY fallback.
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        return Ok(key);
    }
    // 3. For OAuth-backed providers (copilot), try the OAuth token store.
    if provider == "copilot"
        && let Ok(token) = resolve_oauth_token(provider)
    {
        return Ok(token);
    }
    bail!("no API key found for provider '{provider}': set {env_name} or OPENAI_API_KEY")
}

/// Load an OAuth token from the token store for providers that use OAuth.
fn resolve_oauth_token(provider: &str) -> Result<String> {
    use crate::oauth::OAuthTokenStore;
    use crate::paths::{PraxisPaths, default_data_dir};
    let data_dir = default_data_dir()?;
    let paths = PraxisPaths::for_data_dir(data_dir);
    let store = OAuthTokenStore::new(&paths.data_dir);
    let token = store.get(provider)?.with_context(|| {
        format!("no OAuth token for '{provider}' — run `praxis oauth login {provider}`")
    })?;
    if token.is_expired() {
        bail!(
            "OAuth token for '{provider}' has expired — run `praxis oauth login {provider}` or `praxis oauth refresh {provider}`"
        );
    }
    Ok(token.access_token)
}

pub(super) fn resolve_endpoint(route: &ProviderRoute) -> String {
    match &route.base_url {
        Some(base) => {
            // If the operator already specified the full path, use it as-is.
            // Otherwise, append the standard Chat Completions path.
            let base = base.trim_end_matches('/');
            if base.ends_with("/chat/completions") {
                base.to_string()
            } else {
                format!("{base}/v1/chat/completions")
            }
        }
        None => OPENAI_CHAT_URL.to_string(),
    }
}

fn stubbed(route: &ProviderRoute, phase: &str) -> Result<Option<ProviderResponse>> {
    // Provider-specific stub: PRAXIS_<UPPER>_STUB_RESPONSE / PRAXIS_<UPPER>_FORCE_ERROR
    let upper = route.provider.to_uppercase().replace('-', "_");
    let force_error_key = format!("PRAXIS_{upper}_FORCE_ERROR");
    let stub_key = format!("PRAXIS_{upper}_STUB_RESPONSE");

    // Also accept the legacy PRAXIS_OPENAI_* keys for the "openai" provider.
    if let Ok(error) =
        std::env::var(&force_error_key).or_else(|_| std::env::var("PRAXIS_OPENAI_FORCE_ERROR"))
    {
        bail!("{error}");
    }

    Ok(std::env::var(&stub_key)
        .or_else(|_| std::env::var("PRAXIS_OPENAI_STUB_RESPONSE"))
        .ok()
        .map(|summary| ProviderResponse {
            summary,
            attempt: successful_attempt(phase, &route.provider, &route.model, 0, 0, 0),
        }))
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
}
