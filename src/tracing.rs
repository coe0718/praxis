//! OpenTelemetry integration — Distributed tracing pipeline.
//!
//! Moltis uses OpenTelemetry tracing. Praxis currently has Prometheus only.
//! This adds distributed trace support.

use anyhow::Result;

/// Tracer for distributed operations.
pub struct Tracer {
    /// Service name for traces.
    pub service_name: String,
}

impl Tracer {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
        }
    }

    /// Start a new span (placeholder).
    pub fn start_span(&self, _name: &str) -> Span {
        Span {
            start_time: std::time::Instant::now(),
        }
    }

    /// Export traces (placeholder).
    pub fn export(&self, _traces: &[Span]) -> Result<()> {
        Ok(())
    }
}

/// A tracing span.
#[derive(Debug, Clone)]
pub struct Span {
    pub start_time: std::time::Instant,
}

/// Global tracer instance.
pub static GLOBAL_TRACER: std::sync::OnceLock<Tracer> = std::sync::OnceLock::new();

/// Get or initialize the global tracer.
pub fn get_tracer() -> &'static Tracer {
    GLOBAL_TRACER.get_or_init(|| Tracer::new("praxis"))
}