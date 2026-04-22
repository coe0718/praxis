use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::{ProviderRequest, ProviderResponse, successful_attempt};

const OLLAMA_URL: &str = "http://127.0.0.1:11434/api/generate";

pub(super) fn execute(
    route: &ProviderRoute,
    request: &ProviderRequest,
) -> Result<ProviderResponse> {
    if let Some(response) = stubbed(route, request.phase)? {
        return Ok(response);
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .context("failed to build Ollama HTTP client")?;
    let response = client
        .post(route.base_url.as_deref().unwrap_or(OLLAMA_URL))
        .json(&OllamaRequest {
            model: route.model.clone(),
            prompt: request.input.clone(),
            system: request.system.clone(),
            stream: false,
        })
        .send()
        .context("failed to call Ollama provider")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        let safe_body = body.chars().take(200).collect::<String>();
        bail!("Ollama provider request failed with {status}: {safe_body}");
    }

    let parsed = response
        .json::<OllamaResponse>()
        .context("failed to parse Ollama provider response")?;
    if parsed.response.trim().is_empty() {
        bail!("Ollama provider returned no text content");
    }

    Ok(ProviderResponse {
        summary: parsed.response.trim().to_string(),
        attempt: successful_attempt(
            request.phase,
            "ollama",
            &route.model,
            parsed.prompt_eval_count.unwrap_or(0),
            parsed.eval_count.unwrap_or(0),
            route.estimated_cost_micros(
                parsed.prompt_eval_count.unwrap_or(0),
                parsed.eval_count.unwrap_or(0),
            ),
        ),
    })
}

fn stubbed(route: &ProviderRoute, phase: &str) -> Result<Option<ProviderResponse>> {
    if let Ok(error) = std::env::var("PRAXIS_OLLAMA_FORCE_ERROR") {
        bail!("{error}");
    }
    Ok(std::env::var("PRAXIS_OLLAMA_STUB_RESPONSE")
        .ok()
        .map(|summary| ProviderResponse {
            summary,
            attempt: successful_attempt(phase, "ollama", &route.model, 0, 0, 0),
        }))
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    prompt_eval_count: Option<i64>,
    eval_count: Option<i64>,
}
