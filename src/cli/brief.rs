use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::{
    brief::generate_brief,
    paths::{PraxisPaths, default_data_dir},
    time::{Clock, SystemClock},
};

#[derive(Debug, Args)]
pub struct BriefArgs {
    // future: --format json|text
}

pub(crate) fn handle_brief(data_dir_override: Option<PathBuf>, _args: BriefArgs) -> Result<String> {
    let data_dir = data_dir_override.unwrap_or(default_data_dir()?);
    let paths = PraxisPaths::for_data_dir(data_dir);
    let now = SystemClock::from_env()?.now_utc();
    generate_brief(&paths, now)
}
