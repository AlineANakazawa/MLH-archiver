use env_logger::Env;

use anonymizer::Result;
use anonymizer::config;
use anonymizer::start;

fn main() -> Result<()> {
    let env = Env::default().filter_or("RUST_LOG", "info");
    env_logger::init_from_env(env);

    log::info!("anonymizer starting — build: {}", env!("CARGO_PKG_VERSION"));

    let mut app_config = match config::read_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Configuration options:");
            eprintln!("  - Config file:  anonymizer_config.yaml (or similar)");
            eprintln!();
            eprintln!("Run with --help for more information.");
            std::process::exit(1);
        }
    };

    println!("Starting with {} threads", app_config.nthreads);

    rayon::ThreadPoolBuilder::new()
        .num_threads(app_config.nthreads as usize)
        .build_global()
        .unwrap();

    start(&mut app_config)
}
