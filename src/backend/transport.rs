//! Transport ABC — pluggable provider transport layer.
//!
//! Provides a `Transport` trait that abstracts the HTTP request/response cycle
//! for LLM providers. Each provider (Anthropic, OpenAI, Ollama, etc.) implements
//! this trait to handle its specific wire format while sharing common
//! authentication, retry, and error-handling logic.

use anyhow::Result;

use crate::backend::ProviderRequest;
use crate::providers::ProviderRoute;
use crate::usage::ProviderAttempt;

/// A completed transport response with provider-agnostic fields.
#[derive(Debug, Clone)]
pub struct TransportResponse {
    /// The text content returned by the provider.
    pub content: String,
    /// Token counts reported by the provider.
    pub input_tokens: i64,
    pub output_tokens: i64,
    /// Estimated cost in microdollars (1 USD = 1_000_000 microdollars).
    pub estimated_cost_micros: i64,
}

impl TransportResponse {
    /// Convert to a `ProviderAttempt` for usage tracking.
    pub fn into_attempt(self, phase: &str, provider: &str, model: &str) -> ProviderAttempt {
        ProviderAttempt {
            phase: phase.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            estimated_cost_micros: self.estimated_cost_micros,
            success: true,
            error: None,
        }
    }
}

/// Pluggable transport trait for LLM provider HTTP communication.
///
/// Each provider implements `Transport` to handle:
/// - Request serialization (provider-specific JSON format)
/// - Authentication (API key injection, header management)
/// - Response parsing (provider-specific response shape)
/// - Error detection and retry signaling
///
/// The trait is object-safe so transports can be swapped at runtime.
pub trait Transport: Send + Sync {
    /// Human-readable transport name (e.g. "anthropic", "openai", "ollama").
    fn name(&self) -> &'static str;

    /// Send a provider request and return the parsed response.
    ///
    /// Implementations should:
    /// 1. Serialize the request into the provider's wire format
    /// 2. Attach authentication headers from the route's API key
    /// 3. Send the HTTP request with appropriate timeout
    /// 4. Parse the response into a `TransportResponse`
    /// 5. Return an error if the provider signals rate-limiting or failure
    fn send(&self, route: &ProviderRoute, request: &ProviderRequest) -> Result<TransportResponse>;

    /// Check if a given error should trigger a retry.
    ///
    /// Default implementation retries on HTTP 429 and 5xx errors.
    fn should_retry(&self, error: &anyhow::Error) -> bool {
        let msg = format!("{error:#}");
        msg.contains("429") || msg.contains("rate") || msg.contains("502") || msg.contains("503")
    }

    /// Maximum number of retries for this transport.
    fn max_retries(&self) -> u32 {
        2
    }
}

/// Built-in transport registry — maps provider names to transport instances.
pub struct TransportRegistry {
    transports: Vec<(&'static str, Box<dyn Transport>)>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self { transports: Vec::new() }
    }

    /// Register a transport for a provider name.
    pub fn register(&mut self, provider: &'static str, transport: impl Transport + 'static) {
        self.transports.push((provider, Box::new(transport)));
    }

    /// Look up a transport by provider name.
    pub fn get(&self, provider: &str) -> Option<&dyn Transport> {
        self.transports
            .iter()
            .find(|(name, _)| *name == provider)
            .map(|(_, t)| t.as_ref())
    }

    /// List all registered provider names.
    pub fn providers(&self) -> Vec<&'static str> {
        self.transports.iter().map(|(name, _)| *name).collect()
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}
