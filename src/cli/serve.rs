use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::dashboard::serve_dashboard;

use super::core::load_initialized_config;

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 8787)]
    port: u16,
}

pub(crate) fn handle_serve(data_dir_override: Option<PathBuf>, args: ServeArgs) -> Result<String> {
    let (_, paths) = load_initialized_config(data_dir_override)?;
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(serve_dashboard(paths.data_dir, args.host, args.port))?;
    Ok("server: stopped".to_string())
}
