//! Image generation tool — pluggable backends (OpenAI DALL-E, Stability AI, etc.).
//!
//! Dispatched from the `image` tool in `execute.rs`. Supports multiple
//! generation backends selected via `params.provider`:
//! - `openai` (default) — DALL-E 3 via OpenAI API
//! - `stability` — Stable Diffusion via Stability AI API
//! - `fal` — Fal.ai image generation
//!
//! Parameters:
//! - `params.prompt` (required) — text description of the image
//! - `params.size` (optional) — "256x256", "512x512", or "1024x1024" (default)
//! - `params.n` (optional) — number of images, 1-4 (default 1)
//! - `params.provider` (optional) — backend: "openai" (default), "stability", "fal"
//! - `params.model` (optional) — model override (e.g. "dall-e-3", "stable-diffusion-xl")

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::backend::resolve_api_key;
use crate::paths::PraxisPaths;
use crate::storage::StoredApprovalRequest;
use crate::tools::request::parse_payload;

/// OpenAI's image generation endpoint.
const IMAGE_URL: &str = "https://api.openai.com/v1/images/generations";

/// Valid sizes for DALL-E.
const VALID_SIZES: &[&str] = &["256x256", "512x512", "1024x1024"];

#[derive(Debug, Deserialize)]
struct ImageResponse {
    data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    url: Option<String>,
    b64_json: Option<String>,
    revised_prompt: Option<String>,
}

/// Execute the image generation tool.  Returns a summary with URLs for each
/// generated image.
pub fn execute_image_tool(
    _paths: &PraxisPaths,
    request: &StoredApprovalRequest,
) -> Result<crate::tools::ToolExecutionResult> {
    let payload = parse_payload(request.payload_json.as_deref())?;

    let prompt = payload
        .params
        .get("prompt")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("image tool requires params.prompt"))?;

    let size = payload
        .params
        .get("size")
        .map(|s| s.as_str())
        .unwrap_or("1024x1024")
        .to_string();

    if !VALID_SIZES.contains(&size.as_str()) {
        bail!("image tool: invalid size '{size}'. Valid sizes: {}", VALID_SIZES.join(", "));
    }

    let n: u8 = payload.params.get("n").and_then(|s| s.parse().ok()).unwrap_or(1).clamp(1, 4);

    let model = payload.params.get("model").cloned().unwrap_or_else(|| "dall-e-3".to_string());

    // Resolve key using the same path that chat uses — supports pooling.
    let provider = payload.params.get("provider").cloned().unwrap_or_else(|| "openai".to_string());
    let api_key = resolve_api_key(&provider)?;

    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "n": n,
        "size": size,
        "response_format": "url",
    });

    // Select endpoint based on provider.
    let endpoint = match provider.as_str() {
        "openai" | "dall-e" => IMAGE_URL.to_string(),
        "stability" => {
            "https://api.stability.ai/v1/generation/stable-diffusion-xl-1024-v1-0/text-to-image"
                .to_string()
        }
        "fal" => "https://queue.fal.run/fal-ai/fast-sdxl".to_string(),
        other => bail!("image tool: unknown provider '{other}'. Supported: openai, stability, fal"),
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("failed to build HTTP client for image generation")?;

    let response = client
        .post(&endpoint)
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .context("failed to call image generation API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        let safe_body = body.chars().take(300).collect::<String>();
        bail!("image generation failed with {status}: {safe_body}");
    }

    let parsed: ImageResponse =
        response.json().context("failed to parse image generation response")?;

    if parsed.data.is_empty() {
        bail!("image generation returned no images");
    }

    let urls: Vec<String> = parsed
        .data
        .iter()
        .enumerate()
        .map(|(i, img)| {
            let url = img.url.as_deref().or(img.b64_json.as_deref()).unwrap_or("(no url)");
            let note = img
                .revised_prompt
                .as_ref()
                .map(|rp| {
                    format!(" (revised prompt: \"{}\")", rp.chars().take(80).collect::<String>())
                })
                .unwrap_or_default();
            format!("[{}] {}{}", i + 1, url, note)
        })
        .collect();

    let summary = format!(
        "Generated {} image(s) for prompt: \"{}\".\n{}",
        parsed.data.len(),
        prompt,
        urls.join("\n"),
    );

    Ok(crate::tools::ToolExecutionResult { summary })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_sizes_accepted() {
        assert!(VALID_SIZES.contains(&"256x256"));
        assert!(VALID_SIZES.contains(&"512x512"));
        assert!(VALID_SIZES.contains(&"1024x1024"));
        assert!(!VALID_SIZES.contains(&"2048x2048"));
    }
}
