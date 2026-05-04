use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::{
    lite::{clear_model_override, get_model_override, set_model_override},
    paths::default_data_dir,
};

#[derive(Debug, Args)]
pub struct ModelArgs {
    /// Model to switch to, in provider/model format (e.g. anthropic/claude-3-5-sonnet-latest).
    /// Omit to show the current override.
    pub model: Option<String>,

    /// Remove the active model override.
    #[arg(long)]
    pub clear: bool,
}

pub fn handle_model(data_dir_override: Option<PathBuf>, args: ModelArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or_else(|| default_data_dir().expect("no data dir"))?;

    if args.clear {
        clear_model_override(&data_dir)?;
        return Ok("model override cleared".to_string());
    }

    if let Some(model) = args.model {
        set_model_override(&data_dir, &model)?;
        Ok(format!("model override set to '{model}'"))
    } else {
        match get_model_override(&data_dir) {
            Some(current) => Ok(format!("active model override: {current}")),
            None => Ok("no model override active".to_string()),
        }
    }
}
