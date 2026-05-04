use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::paths::default_data_dir;

// ── CLI types ──────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelCommand,
}

#[derive(Debug, Subcommand)]
pub enum ModelCommand {
    /// List locally known models (from providers.toml or defaults).
    List,
    /// Fetch the latest model catalog from OpenRouter and cache it locally.
    Fetch,
    /// Search the cached catalog for models matching a query.
    Search {
        /// Search query — matched against model id and name (case-insensitive).
        query: String,
    },
}

// ── Cached catalog types ──────────────────────────────────────────────────

/// Wrapper persisted to disk with a TTL timestamp.
#[derive(Debug, Serialize, Deserialize)]
pub struct CachedCatalog {
    /// When this catalog was fetched.
    pub fetched_at: DateTime<Utc>,
    /// Seconds after `fetched_at` before the cache is considered stale.
    pub ttl_secs: u64,
    /// Model entries from the remote API.
    pub models: Vec<CatalogEntry>,
}

/// A single model entry from the OpenRouter `/api/v1/models` response.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub context_length: Option<u64>,
    #[serde(default)]
    pub pricing: Option<ModelPricing>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelPricing {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub completion: Option<String>,
}

/// Top-level response shape from OpenRouter `/api/v1/models`.
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<u64>,
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

// ── Constants ─────────────────────────────────────────────────────────────

const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const DEFAULT_TTL_SECS: u64 = 6 * 3600; // 6 hours

// ── Handler ───────────────────────────────────────────────────────────────

pub fn handle_models(data_dir_override: Option<PathBuf>, args: ModelsArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    match args.command {
        ModelCommand::List => handle_list(&data_dir),
        ModelCommand::Fetch => handle_fetch(&data_dir),
        ModelCommand::Search { query } => handle_search(&data_dir, &query),
    }
}

// ── list ───────────────────────────────────────────────────────────────────

fn handle_list(data_dir: &std::path::Path) -> Result<String> {
    // Load local provider routes.
    let providers_path = data_dir.join("providers.toml");
    let settings = crate::providers::ProviderSettings::load_or_default(&providers_path)?;

    let mut lines: Vec<String> = Vec::new();
    lines.push("Locally configured models (providers.toml):\n".to_string());

    for route in &settings.providers {
        let cost_info = match (route.input_cost_per_million_usd, route.output_cost_per_million_usd)
        {
            (Some(i), Some(o)) => format!("  in=${i:.2}/M  out=${o:.2}/M"),
            _ => "  (no pricing configured)".to_string(),
        };
        lines.push(format!("  {}/{}{}", route.provider, route.model, cost_info));
    }

    // Also mention the cached catalog if it exists.
    let catalog_path = catalog_path(data_dir);
    if catalog_path.exists() {
        match load_catalog(&catalog_path) {
            Ok(cached) => {
                let age = (Utc::now() - cached.fetched_at).num_seconds();
                let fresh = age < cached.ttl_secs as i64;
                lines.push(format!(
                    "\nCached OpenRouter catalog: {} models (fetched {}s ago, {})",
                    cached.models.len(),
                    age,
                    if fresh { "fresh" } else { "stale" }
                ));
            }
            Err(_) => {
                lines.push("\nCached catalog exists but could not be parsed.".to_string());
            }
        }
    } else {
        lines.push(
            "\nNo cached OpenRouter catalog. Run `praxis models fetch` to download one."
                .to_string(),
        );
    }

    Ok(lines.join("\n"))
}

// ── fetch ──────────────────────────────────────────────────────────────────

fn handle_fetch(data_dir: &std::path::Path) -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    let resp = client
        .get(OPENROUTER_MODELS_URL)
        .send()
        .context("failed to reach OpenRouter API")?;

    if !resp.status().is_success() {
        bail!("OpenRouter returned HTTP {}", resp.status());
    }

    let body: OpenRouterResponse = resp.json().context("failed to parse OpenRouter response")?;

    let entries: Vec<CatalogEntry> = body
        .data
        .into_iter()
        .map(|m| CatalogEntry {
            id: m.id,
            name: m.name.unwrap_or_default(),
            description: m.description,
            context_length: m.context_length,
            pricing: m.pricing.map(|p| ModelPricing {
                prompt: p.prompt,
                completion: p.completion,
            }),
        })
        .collect();

    let count = entries.len();
    let catalog = CachedCatalog {
        fetched_at: Utc::now(),
        ttl_secs: DEFAULT_TTL_SECS,
        models: entries,
    };

    let path = catalog_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&catalog).context("failed to serialize catalog")?;
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(format!("fetched {count} models → {}", path.display()))
}

// ── search ─────────────────────────────────────────────────────────────────

fn handle_search(data_dir: &std::path::Path, query: &str) -> Result<String> {
    let path = catalog_path(data_dir);
    if !path.exists() {
        bail!(
            "no cached catalog found. Run `praxis models fetch` first.\n  expected at: {}",
            path.display()
        );
    }

    let cached = load_catalog(&path)?;
    let age = (Utc::now() - cached.fetched_at).num_seconds();
    let fresh = age < cached.ttl_secs as i64;

    let q = query.to_lowercase();
    let matches: Vec<&CatalogEntry> = cached
        .models
        .iter()
        .filter(|m| m.id.to_lowercase().contains(&q) || m.name.to_lowercase().contains(&q))
        .collect();

    if matches.is_empty() {
        return Ok(format!(
            "no models matching '{query}' (searched {} models)",
            cached.models.len()
        ));
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "found {} match(es) for '{query}' (searched {} models, {}):\n",
        matches.len(),
        cached.models.len(),
        if fresh {
            "cache fresh"
        } else {
            "cache stale — consider `praxis models fetch`"
        },
    ));

    for m in matches.iter().take(50) {
        let ctx = m.context_length.map(|c| format!("ctx={c}")).unwrap_or_default();
        let price = m
            .pricing
            .as_ref()
            .and_then(|p| p.prompt.as_ref().map(|pr| format!("in=${pr}")))
            .unwrap_or_default();
        lines.push(format!("  {} — {}  {ctx} {price}", m.id, m.name));
    }

    if matches.len() > 50 {
        lines.push(format!("  … and {} more", matches.len() - 50));
    }

    Ok(lines.join("\n"))
}

// ── helpers ────────────────────────────────────────────────────────────────

fn catalog_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("model_catalog.json")
}

fn load_catalog(path: &std::path::Path) -> Result<CachedCatalog> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}
