use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::{ProviderRequest, ProviderResponse, successful_attempt};

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_BETA_CACHE: &str = "prompt-caching-2024-07-31";

/// Minimum input tokens before we bother enabling prompt caching.
/// Below this threshold the caching overhead is not worth it.
const CACHE_MIN_TOKENS: u32 = 1024;

pub(super) fn execute(
    route: &ProviderRoute,
    request: &ProviderRequest,
    prompt_caching: bool,
) -> Result<ProviderResponse> {
    if let Some(response) = stubbed(route, request.phase)? {
        return Ok(response);
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY is required for the Claude provider")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build Claude HTTP client")?;

    // Enable prompt caching when the operator has opted in and the request is
    // large enough to benefit.  The `cache_control` block on the system prompt
    // causes Anthropic to cache everything up to and including that turn, so
    // repeated Orient / Ask calls with the same system text pay only for the
    // delta.
    let use_cache = prompt_caching && request.max_output_tokens >= CACHE_MIN_TOKENS;

    let system = if use_cache {
        ClaudeSystem::Blocks(vec![ClaudeSystemBlock {
            kind: "text",
            text: request.system.clone(),
            cache_control: Some(CacheControl { kind: "ephemeral" }),
        }])
    } else {
        ClaudeSystem::Plain(request.system.clone())
    };

    let mut req_builder = client
        .post(ANTHROPIC_URL)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("x-api-key", &api_key);

    if use_cache {
        req_builder = req_builder.header("anthropic-beta", ANTHROPIC_BETA_CACHE);
    }

    let response = req_builder
        .json(&ClaudeRequest {
            model: route.model.clone(),
            max_tokens: request.max_output_tokens,
            system,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: request.input.clone(),
            }],
        })
        .send()
        .context("failed to call Claude provider")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("Claude provider request failed with {status}: {body}");
    }

    let parsed = response
        .json::<ClaudeResponse>()
        .context("failed to parse Claude provider response")?;
    let text = parsed
        .content
        .into_iter()
        .find_map(|block| match block {
            ClaudeContent::Text { text } => Some(text.trim().to_string()),
            ClaudeContent::Other => None,
        })
        .filter(|text| !text.is_empty())
        .context("Claude provider returned no text content")?;

    let input_tokens = parsed
        .usage
        .as_ref()
        .map(|u| u.input_tokens + u.cache_read_input_tokens.unwrap_or(0))
        .unwrap_or(0);
    let output_tokens = parsed
        .usage
        .as_ref()
        .map(|u| u.output_tokens)
        .unwrap_or(0);

    Ok(ProviderResponse {
        summary: text,
        attempt: successful_attempt(
            request.phase,
            "claude",
            &route.model,
            input_tokens,
            output_tokens,
            route.estimated_cost_micros(input_tokens, output_tokens),
        ),
    })
}

fn stubbed(route: &ProviderRoute, phase: &str) -> Result<Option<ProviderResponse>> {
    if let Ok(error) = std::env::var("PRAXIS_CLAUDE_FORCE_ERROR") {
        bail!("{error}");
    }
    Ok(std::env::var("PRAXIS_CLAUDE_STUB_RESPONSE")
        .ok()
        .map(|summary| ProviderResponse {
            summary,
            attempt: successful_attempt(phase, "claude", &route.model, 0, 0, 0),
        }))
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: ClaudeSystem,
    messages: Vec<ClaudeMessage>,
}

/// System prompt: either a plain string or a list of typed blocks (required for
/// prompt caching).
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ClaudeSystem {
    Plain(String),
    Blocks(Vec<ClaudeSystemBlock>),
}

#[derive(Debug, Serialize)]
struct ClaudeSystemBlock {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize)]
struct CacheControl {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: i64,
    output_tokens: i64,
    /// Tokens read from the prompt cache (counts toward billed input tokens at
    /// the discounted cache-read rate).
    #[serde(default)]
    cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClaudeContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}
