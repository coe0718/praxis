use std::collections::HashSet;

use anyhow::{Result, bail};

use crate::time::{parse_clock_time, parse_timezone};

use super::AppConfig;

impl AppConfig {
    pub fn validate(&self) -> Result<()> {
        if self.instance.name.trim().is_empty() {
            bail!("instance.name must not be empty");
        }

        parse_timezone(&self.instance.timezone)?;
        parse_clock_time(&self.runtime.quiet_hours_start)?;
        parse_clock_time(&self.runtime.quiet_hours_end)?;

        if !(1..=4).contains(&self.security.level) {
            bail!("security.level must be between 1 and 4");
        }

        if !matches!(
            self.agent.backend.as_str(),
            "stub" | "claude" | "openai" | "ollama" | "router"
        ) {
            bail!(
                "agent.backend must be one of \"stub\", \"claude\", \"openai\", \"ollama\", or \"router\", got {}",
                self.agent.backend
            );
        }

        if self.agent.profile.trim().is_empty() {
            bail!("agent.profile must not be empty");
        }

        if !(0.0..=1.0).contains(&self.agent.context_ceiling_pct)
            || self.agent.context_ceiling_pct == 0.0
        {
            bail!("agent.context_ceiling_pct must be greater than 0.0 and at most 1.0");
        }

        if matches!(self.agent.backend.as_str(), "claude" | "openai")
            && self
                .agent
                .model_pin
                .as_deref()
                .is_some_and(|model| model.trim().is_empty())
        {
            bail!("agent.model_pin must not be blank when provided");
        }

        if self.database.path.as_os_str().is_empty() {
            bail!("database.path must not be empty");
        }

        if self.runtime.state_file.as_os_str().is_empty() {
            bail!("runtime.state_file must not be empty");
        }

        if self.context.window_tokens == 0 {
            bail!("context.window_tokens must be greater than 0");
        }

        if self.context.budget.is_empty() {
            bail!("context.budget must not be empty");
        }

        let mut priorities = HashSet::new();
        for source in &self.context.budget {
            if source.source.trim().is_empty() {
                bail!("context.budget entries must have a source name");
            }

            if !(0.0..=1.0).contains(&source.max_pct) || source.max_pct == 0.0 {
                bail!("context budget percentages must be greater than 0.0 and at most 1.0");
            }

            if !priorities.insert(source.priority) {
                bail!("context.budget priorities must be unique");
            }
        }

        Ok(())
    }
}
