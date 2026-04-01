fn main() {
    if let Err(error) = praxis::cli::run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
