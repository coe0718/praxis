use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::{paths::default_data_dir, tui::run_tui};

#[derive(Debug, Args)]
pub struct TuiArgs {
    // no options yet — future: --refresh-ms
}

pub(crate) fn handle_tui(data_dir_override: Option<PathBuf>, _args: TuiArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    run_tui(data_dir)?;
    Ok("tui: closed".to_string())
}
