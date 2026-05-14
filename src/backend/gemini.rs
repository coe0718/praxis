//! Google Gemini API provider adapter.
//!
//! Gemini uses a different API format than OpenAI.
//! Requires `PRAXIS_GEMINI_API_KEY` environment variable.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::{InputContent, ProviderRequest, ProviderResponse, successful_attempt};

const GEMINI_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

pub(super) fn execute(
    route: &ProviderRoute,
    request: &ProviderRequest,
) -> Result<ProviderResponse> {
    if let Some(response) = stubbed(route, request.phase)? {
        return Ok(response);
    }

    let api_key = std::env::var("PRAXIS_GEMINI_API_KEY")
        .context("PRAXIS_GEMINI_API_KEY is required for the Gemini provider")?;

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build Gemini HTTP client")?;

    let url = format!("{}/models/{}:generateContent?key={}", GEMINI_URL, route.model, api_key);

    // Convert input to text for Gemini
    let user_text = match &request.input {
        InputContent::Text(text) => text.clone(),
        InputContent::Blocks(blocks) => {
            serde_json::to_string(blocks).context("failed to serialize content blocks")?
        }
    };

    let response = client
        .post(&url)
        .json(&GeminiRequest {
            system_instruction: Some(GeminiSystemInstruction {
                parts: vec![GeminiTextPart { text: request.system.clone() }],
            }),
            contents: vec![GeminiContent {
                role: "user".to_string(),
                parts: vec![GeminiTextPart { text: user_text }],
            }],
            generation_config: GeminiGenerationConfig {
                max_output_tokens: request.max_output_tokens,
            },
        })
        .send()
        .context("failed to call Gemini provider")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("Gemini provider request failed with {status}: {body}");
    }

    let parsed = response
        .json::<GeminiResponse>()
        .context("failed to parse Gemini provider response")?;

    let text = parsed
        .candidates
        .into_iter()
        .find_map(|c| c.content.parts.into_iter().find_map(|p| p.text))
        .context("Gemini provider returned no text content")?;

    Ok(ProviderResponse {
        summary: text,
        attempt: successful_attempt(
            request.phase,
            "gemini",
            &route.model,
            0, // token counting not implemented
            0,
            0,
        ),
    })
}

fn stubbed(_route: &ProviderRoute, phase: &str) -> Result<Option<ProviderResponse>> {
    Ok(std::env::var("PRAXIS_GEMINI_STUB_RESPONSE")
        .ok()
        .map(|summary| ProviderResponse {
            summary,
            attempt: successful_attempt(phase, "gemini", "stub", 0, 0, 0),
        }))
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct GeminiRequest {
    system_instruction: Option<GeminiSystemInstruction>,
    contents: Vec<GeminiContent>,
    generation_config: GeminiGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GeminiSystemInstruction {
    parts: Vec<GeminiTextPart>,
}

#[derive(Debug, Serialize)]
struct GeminiTextPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiTextPart>,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
}

#[derive(Debug, Deserialize)]
struct GeminiResponseContent {
    parts: Vec<GeminiResponsePart>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}
