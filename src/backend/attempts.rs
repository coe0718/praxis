use anyhow::Error;

use crate::{providers::ProviderRoute, usage::ProviderAttempt};

pub(super) fn successful_attempt(
    phase: &str,
    provider: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    estimated_cost_micros: i64,
) -> ProviderAttempt {
    ProviderAttempt {
        phase: phase.to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        success: true,
        input_tokens,
        output_tokens,
        estimated_cost_micros,
        error: None,
    }
}

pub(super) fn failed_attempt(route: &ProviderRoute, phase: &str, error: Error) -> ProviderAttempt {
    ProviderAttempt {
        phase: phase.to_string(),
        provider: route.provider.clone(),
        model: route.model.clone(),
        success: false,
        input_tokens: 0,
        output_tokens: 0,
        estimated_cost_micros: 0,
        error: Some(error.to_string()),
    }
}
