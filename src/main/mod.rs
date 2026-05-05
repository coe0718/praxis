fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    if let Err(error) = praxis::cli::run() {
        log::error!("{error:#}");
        std::process::exit(1);
    }
}
