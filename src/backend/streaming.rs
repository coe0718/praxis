//! Async streaming LLM backend.
//!
//! Provides token-by-token streaming responses using async `reqwest` and
//! SSE parsing.  This is designed to coexist with the existing blocking
//! backend — the runtime can choose streaming for dashboard sessions and
//! blocking for CLI / background tasks.

use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::providers::ProviderRoute;

use super::ProviderRequest;
use super::openai::{resolve_api_key, resolve_endpoint};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

/// A single streamed token (or the final usage block).
#[derive(Debug, Clone)]
pub enum StreamEvent {
    Token(String),
    Usage {
        input_tokens: i64,
        output_tokens: i64,
    },
    Done,
}

/// Async streaming backend for OpenAI-compatible providers.
pub struct StreamingBackend {
    client: Client,
}

impl StreamingBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .context("failed to build async HTTP client")?,
        })
    }

    /// Stream completion tokens for the given request.
    ///
    /// Yields `StreamEvent::Token` for each delta, then
    /// `StreamEvent::Usage` with final counts, then `StreamEvent::Done`.
    pub fn stream_completion<'a>(
        &'a self,
        route: &'a ProviderRoute,
        request: &'a ProviderRequest,
    ) -> impl Stream<Item = Result<StreamEvent>> + Send + 'a {
        async_stream::try_stream! {
            let api_key = resolve_api_key(&route.provider)?;
            let endpoint = resolve_endpoint(route);

            let body = StreamChatRequest {
                model: route.model.clone(),
                messages: vec![
                    StreamChatMessage {
                        role: "system".to_string(),
                        content: request.system.to_string(),
                    },
                    StreamChatMessage {
                        role: "user".to_string(),
                        content: request.input.clone(),
                    },
                ],
                max_completion_tokens: Some(request.max_output_tokens),
                stream: true,
            };

            let response = self
                .client
                .post(&endpoint)
                .bearer_auth(api_key)
                .json(&body)
                .send()
                .await
                .with_context(|| format!("failed to call provider {}", route.provider))?;

            let response = response.error_for_status()
                .with_context(|| format!("provider {} request failed", route.provider))?;

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = StreamExt::next(&mut stream).await {
                let chunk = chunk.context("stream error")?;
                buffer.push_str(&String::from_utf8_lossy(chunk.as_ref()));

                while let Some(pos) = buffer.find("\n\n") {
                    let event = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    for line in event.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                yield StreamEvent::Done;
                                return;
                            }
                            if let Ok(delta) = serde_json::from_str::<StreamDelta>(data) {
                                if let Some(usage) = delta.usage {
                                    yield StreamEvent::Usage {
                                        input_tokens: usage.prompt_tokens,
                                        output_tokens: usage.completion_tokens,
                                    };
                                }
                                if let Some(choice) = delta.choices.into_iter().next() {
                                    if let Some(content) = choice.delta.content {
                                        if !content.is_empty() {
                                            yield StreamEvent::Token(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            yield StreamEvent::Done;
        }
    }
}

// ── Request / Response types for streaming ───────────────────────────────────

#[derive(Debug, Serialize)]
struct StreamChatRequest {
    model: String,
    messages: Vec<StreamChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct StreamChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDeltaContent,
}

#[derive(Debug, Deserialize, Default)]
struct StreamDeltaContent {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamUsage {
    prompt_tokens: i64,
    completion_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_backend_constructs() {
        let backend = StreamingBackend::new();
        assert!(backend.is_ok());
    }
}
