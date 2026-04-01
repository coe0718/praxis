use std::collections::HashMap;

use crate::config::{AppConfig, ContextSourceConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSourceInput {
    pub source: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetedSource {
    pub source: String,
    pub content: String,
    pub used_tokens: usize,
    pub summarized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetedContext {
    pub total_budget_tokens: usize,
    pub used_tokens: usize,
    pub included_sources: Vec<BudgetedSource>,
    pub dropped_sources: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ContextBudgeter;

impl ContextBudgeter {
    pub fn allocate(&self, config: &AppConfig, inputs: Vec<ContextSourceInput>) -> BudgetedContext {
        let total_budget_tokens = total_budget_tokens(config);
        let mut remaining_tokens = total_budget_tokens;
        let mut included_sources = Vec::new();
        let mut dropped_sources = Vec::new();
        let input_map = inputs
            .into_iter()
            .map(|input| (input.source.clone(), input))
            .collect::<HashMap<_, _>>();

        let mut budget = config.context.budget.clone();
        budget.sort_by_key(|entry| entry.priority);

        for entry in budget {
            let Some(input) = input_map.get(&entry.source) else {
                continue;
            };
            if input.content.trim().is_empty() {
                continue;
            }

            if remaining_tokens == 0 {
                dropped_sources.push(entry.source.clone());
                continue;
            }

            let source_cap = max_tokens_for(total_budget_tokens, &entry).min(remaining_tokens);
            if source_cap == 0 {
                dropped_sources.push(entry.source.clone());
                continue;
            }

            let estimated_tokens = estimate_tokens(&input.content);
            let (content, summarized) = if estimated_tokens > source_cap {
                (truncate_to_tokens(&input.content, source_cap), true)
            } else {
                (input.content.clone(), false)
            };

            let used_tokens = estimate_tokens(&content);
            if used_tokens == 0 {
                continue;
            }

            remaining_tokens = remaining_tokens.saturating_sub(used_tokens);
            included_sources.push(BudgetedSource {
                source: entry.source,
                content,
                used_tokens,
                summarized,
            });
        }

        BudgetedContext {
            total_budget_tokens,
            used_tokens: total_budget_tokens.saturating_sub(remaining_tokens),
            included_sources,
            dropped_sources,
        }
    }
}

impl BudgetedContext {
    pub fn render(&self) -> String {
        self.included_sources
            .iter()
            .map(|source| format!("## {}\n{}", source.source, source.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    pub fn summary(&self) -> String {
        let included = self
            .included_sources
            .iter()
            .map(|source| {
                if source.summarized {
                    format!("{} (summarized)", source.source)
                } else {
                    source.source.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let dropped = if self.dropped_sources.is_empty() {
            "none".to_string()
        } else {
            self.dropped_sources.join(", ")
        };

        format!(
            "Context used {}/{} tokens. Included: {}. Dropped: {}.",
            self.used_tokens,
            self.total_budget_tokens,
            if included.is_empty() {
                "none"
            } else {
                &included
            },
            dropped,
        )
    }
}

pub fn estimate_tokens(content: &str) -> usize {
    let chars = content.chars().count();
    if chars == 0 { 0 } else { chars.div_ceil(4) }
}

fn total_budget_tokens(config: &AppConfig) -> usize {
    ((config.context.window_tokens as f32) * config.agent.context_ceiling_pct)
        .round()
        .max(1.0) as usize
}

fn max_tokens_for(total_budget_tokens: usize, source: &ContextSourceConfig) -> usize {
    ((total_budget_tokens as f32) * source.max_pct)
        .round()
        .max(1.0) as usize
}

fn truncate_to_tokens(content: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens.saturating_mul(4);
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    let truncated = content
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{}...", truncated.trim_end())
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    use super::{ContextBudgeter, ContextSourceInput};

    #[test]
    fn allocates_context_by_priority_and_capacity() {
        let config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        let context = ContextBudgeter.allocate(
            &config,
            vec![
                ContextSourceInput {
                    source: "identity".to_string(),
                    content: "I am Praxis".repeat(20),
                },
                ContextSourceInput {
                    source: "task".to_string(),
                    content: "ship a real memory system".repeat(50),
                },
            ],
        );

        assert!(!context.included_sources.is_empty());
        assert!(context.used_tokens <= context.total_budget_tokens);
        assert!(context.render().contains("## identity"));
    }

    #[test]
    fn summarizes_sources_that_exceed_their_cap() {
        let config = AppConfig::default_for_data_dir("/tmp/praxis".into());
        let context = ContextBudgeter.allocate(
            &config,
            vec![ContextSourceInput {
                source: "identity".to_string(),
                content: "A".repeat(10_000),
            }],
        );

        assert!(context.included_sources[0].summarized);
    }
}
