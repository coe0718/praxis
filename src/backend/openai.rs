use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::{ProviderRequest, ProviderResponse, successful_attempt};

const OPENAI_URL: &str = "https://api.openai.com/v1/responses";

pub(super) fn execute(
    route: &ProviderRoute,
    request: &ProviderRequest,
) -> Result<ProviderResponse> {
    if let Some(response) = stubbed(route, request.phase)? {
        return Ok(response);
    }

    let api_key =
        std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is required for OpenAI")?;
    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .context("failed to build OpenAI HTTP client")?;
    let response = client
        .post(route.base_url.as_deref().unwrap_or(OPENAI_URL))
        .bearer_auth(api_key)
        .json(&OpenAiRequest {
            model: route.model.clone(),
            instructions: request.system.to_string(),
            input: request.input.clone(),
            max_output_tokens: request.max_output_tokens,
        })
        .send()
        .context("failed to call OpenAI provider")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("OpenAI provider request failed with {status}: {body}");
    }

    let parsed = response
        .json::<OpenAiResponse>()
        .context("failed to parse OpenAI provider response")?;
    let text = parsed
        .output_text
        .or_else(|| {
            parsed.output.iter().find_map(|item| {
                item.content.iter().find_map(|part| {
                    (part.kind == "output_text").then(|| part.text.trim().to_string())
                })
            })
        })
        .filter(|text| !text.is_empty())
        .context("OpenAI provider returned no text content")?;
    let input_tokens = parsed
        .usage
        .as_ref()
        .map(|usage| usage.input_tokens)
        .unwrap_or(0);
    let output_tokens = parsed
        .usage
        .as_ref()
        .map(|usage| usage.output_tokens)
        .unwrap_or(0);

    Ok(ProviderResponse {
        summary: text,
        attempt: successful_attempt(
            request.phase,
            "openai",
            &route.model,
            input_tokens,
            output_tokens,
            route.estimated_cost_micros(input_tokens, output_tokens),
        ),
    })
}

fn stubbed(route: &ProviderRoute, phase: &str) -> Result<Option<ProviderResponse>> {
    if let Ok(error) = std::env::var("PRAXIS_OPENAI_FORCE_ERROR") {
        bail!("{error}");
    }
    Ok(std::env::var("PRAXIS_OPENAI_STUB_RESPONSE")
        .ok()
        .map(|summary| ProviderResponse {
            summary,
            attempt: successful_attempt(phase, "openai", &route.model, 0, 0, 0),
        }))
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    instructions: String,
    input: String,
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Vec<OpenAiMessage>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    content: Vec<OpenAiContent>,
}

#[derive(Debug, Deserialize)]
struct OpenAiContent {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    input_tokens: i64,
    output_tokens: i64,
}
