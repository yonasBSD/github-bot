mod commands;

use clap::Parser;
use commands::{maintain, merge};
use github_bot_lib::cli::{Args, Commands};

use std::env;
use terminal_banner::Banner;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{
    EnvFilter, Registry, filter::FilterExt, layer::SubscriberExt, prelude::*,
};

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

fn main() -> anyhow::Result<()> {
    const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
    const PROJECT_DESC: &str = env!("CARGO_PKG_DESCRIPTION");

    #[cfg(not(target_arch = "wasm32"))]
    setup_panic!(
        metadata!()
            .authors("Acme Inc. <support@example.com")
            .homepage("www.example.com")
            .support("- Open a support request by email to support@example.com")
    );

    let cli = Args::parse();
    let max_level_filter = LevelFilter::from(cli.verbosity);

    // 1. Define the formatted output (The Layer)
    let telemetry_fmt = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false);

    // 2. Define the first filter (Environment variable)
    let env_filter = EnvFilter::from_default_env();

    // 3. Combine the filters: Apply both the environment filter AND the max level filter.
    // Note: When chaining filters (env_filter.and(max_level_filter)), the filter that
    // allows an event to pass is the intersection of both.
    let combined_filter = env_filter.and(max_level_filter);

    // 4. Construct the registry, applying the format layer and the combined filter layer
    let registry = Registry::default()
        // Apply formatting layer, filtered by the combined filter
        .with(telemetry_fmt.with_filter(combined_filter))
        // Send traces to tokio console
        .with(console_subscriber::spawn());

    tracing::subscriber::set_global_default(registry)?;

    tracing::debug!("Logging initialized!");
    tracing::trace!("Tracing initialized!");
    tracing::debug!("Ready to begin...");

    if std::env::var("RUST_LOG").is_ok()
        && ["debug", "trace"].contains(&std::env::var("RUST_LOG").unwrap().to_lowercase().as_str())
    {
        let banner = Banner::new()
            .text(format!("Welcome to {}!", PROJECT_NAME).into())
            .text(PROJECT_DESC.into())
            .render();

        println!("{banner}");
    }

    // Parse the command-line arguments
    tracing::trace!(
        token = cli.token,
        verbosity = ?cli.verbosity.log_level(),
        command = ?cli.command,
        "Parsed command line arguments"
    );

    match &cli.command {
        Commands::Maintain { repo, action } => {
            maintain::run(repo, action).map_err(anyhow::Error::from)?
        }
        Commands::Merge { repo } => merge::run(repo)?,
    };

    Ok(())
}
