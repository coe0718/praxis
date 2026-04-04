use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::argus::{analyze, render};

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct ArgusArgs {
    #[arg(long, default_value_t = 10)]
    limit: usize,
}

pub(crate) fn handle_argus(data_dir_override: Option<PathBuf>, args: ArgusArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    Ok(render(&analyze(&paths.database_file, args.limit)?))
}
