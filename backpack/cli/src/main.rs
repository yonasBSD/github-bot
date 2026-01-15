mod commands;

use clap::Parser;
use commands::{git, hello, maintain, merge, prune, wip};
use std::env;

use github_bot_lib::cli::{Args, Commands};
use github_bot_lib::log::{self, LogFormat, Printer, ScreenLogger, SimpleLogger, Verbosity};
use github_bot_lib::plugins::{self, Event};

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //
    // ────────────────────────────────────────────────────────────────
    // Panic handler (native only)
    // ────────────────────────────────────────────────────────────────
    //
    #[cfg(not(target_arch = "wasm32"))]
    setup_panic!(
        metadata!()
            .authors("Acme Inc. <support@example.com")
            .homepage("www.example.com")
            .support("- Open a support request by email to support@example.com")
    );

    //
    // ────────────────────────────────────────────────────────────────
    // Load environment files
    // ────────────────────────────────────────────────────────────────
    //
    env_rs::init()?;

    //
    // ────────────────────────────────────────────────────────────────
    // Parse CLI arguments
    // ────────────────────────────────────────────────────────────────
    //
    let mut verbosity = Verbosity::Normal;
    let mut format = LogFormat::Text;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-q" => verbosity = Verbosity::Quiet,
            "-v" => verbosity = Verbosity::Verbose,
            "-vv" => verbosity = Verbosity::Trace,
            "--json" => format = LogFormat::Json,
            _ => {}
        }
    }

    //
    // ────────────────────────────────────────────────────────────────
    // Initialize our new logger
    // ────────────────────────────────────────────────────────────────
    //
    log::init();
    let formatter = SimpleLogger::new(verbosity);
    let logger = Printer::new(formatter, format);

    logger.debug("Logger initialized");
    logger.trace("Tracing enabled");
    logger.debug("Ready to begin");

    //
    // ────────────────────────────────────────────────────────────────
    // Banner (only when RUST_LOG=debug or trace)
    // ────────────────────────────────────────────────────────────────
    //
    const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
    const PROJECT_DESC: &str = env!("CARGO_PKG_DESCRIPTION");

    if std::env::var("RUST_LOG").is_ok()
        && ["debug", "trace"].contains(&std::env::var("RUST_LOG").unwrap().to_lowercase().as_str())
    {
        let banner = Banner::new()
            .text(format!("Welcome to {PROJECT_NAME}!").into())
            .text(PROJECT_DESC.into())
            .render();

        println!("{banner}");
    }

    //
    // ────────────────────────────────────────────────────────────────
    // Parse CLI using your existing struct
    // ────────────────────────────────────────────────────────────────
    //
    let cli = Args::parse();

    logger.trace(&format!(
        "Parsed CLI arguments: token={:?}, command={:?}",
        cli.token, cli.command
    ));

    //
    // ────────────────────────────────────────────────────────────────
    // Plugin Initialization Phase
    // ────────────────────────────────────────────────────────────────
    //
    logger.intro("Initializing plugins");

    plugins::broadcast_event(&[], Event::PluginRegistrationInit).await;

    let plugins = plugins::discover_plugins()?;
    for plugin in &plugins {
        plugins::broadcast_event(
            &plugins,
            plugins::Event::PluginRegistered(plugin.manifest.name.clone()),
        )
        .await;
    }

    plugins::broadcast_event(&plugins, Event::PluginRegistrationEnd).await;

    logger.outro("Plugin registration complete");

    //
    // ────────────────────────────────────────────────────────────────
    // Command dispatch
    // ────────────────────────────────────────────────────────────────
    //
    match &cli.command {
        Commands::Maintain { repo, action } => {
            logger.intro("Running maintain command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            let action_arg = action.clone().unwrap_or_else(|| String::from("none"));
            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "maintain".into(),
                    args: vec![repo.clone(), action_arg],
                },
            )
            .await;

            maintain::run(repo.clone(), action)?;

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Maintain command complete");
        }

        Commands::Merge { repo } => {
            logger.intro("Running merge command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "merge".into(),
                    args: vec![repo.clone()],
                },
            )
            .await;

            merge::run(repo.clone())?;

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Merge command complete");
        }

        Commands::Wip {
            no_push,
            no_diff,
            rewind,
        } => {
            logger.intro("Running wip command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "wip".into(),
                    args: vec![
                        no_push.to_string(),
                        no_diff.to_string(),
                        format!("{:#?}", rewind),
                    ],
                },
            )
            .await;

            wip::run(*no_push, *no_diff, *rewind);

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Wip command complete");
        }

        Commands::Prune { yes } => {
            logger.intro("Running prune command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "prune".into(),
                    args: vec![yes.to_string()],
                },
            )
            .await;

            if let Err(e) = prune::run(*yes) {
                logger.err(&format!("{e}"));
            }

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Prune command complete");
        }

        Commands::Git { command } => {
            logger.intro("Running git command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "git".into(),
                    args: vec![command.to_string()],
                },
            )
            .await;

            git::run()?;

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Git command complete");
        }

        Commands::Hello {} => {
            logger.intro("Running hello command");

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            plugins::broadcast_event(
                &plugins,
                Event::CliCommandExecutionRun {
                    command: "hello".into(),
                    args: vec![],
                },
            )
            .await;

            hello::run()?;

            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;

            logger.outro("Hello command complete");
        }
    }

    logger.ok("All done");
    Ok(())
}

/*
mod commands;

use clap::Parser;
use commands::{git, hello, maintain, merge, prune, wip};
use std::env;
use terminal_banner::Banner;
//use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt, prelude::*};

use github_bot_lib::cli::{Args, Commands};
use github_bot_lib::plugins::{self, Event};

#[cfg(not(target_arch = "wasm32"))]
use human_panic::{metadata, setup_panic};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    setup_panic!(
        metadata!()
            .authors("Acme Inc. <support@example.com")
            .homepage("www.example.com")
            .support("- Open a support request by email to support@example.com")
    );

    // Load .env, .env.$APP_ENV, and .env.local, respectively
    env_rs::init()?;

    const PROJECT_NAME: &str = env!("CARGO_PKG_NAME");
    const PROJECT_DESC: &str = env!("CARGO_PKG_DESCRIPTION");

    let cli = Args::parse();
    //let max_level_filter = LevelFilter::from(cli.verbosity);

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

    // --- This does not work ---
    // 3. Combine the filters: Apply both the environment filter AND the max level filter.
    // Note: When chaining filters (env_filter.and(max_level_filter)), the filter that
    // allows an event to pass is the intersection of both.
    //let combined_filter = env_filter.and(max_level_filter);

    // 4. Construct the registry, applying the format layer and the combined filter layer
    let registry = Registry::default()
        // Apply formatting layer, filtered by the combined filter
        //.with(telemetry_fmt.with_filter(combined_filter))
        .with(telemetry_fmt.with_filter(env_filter))
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
            .text(format!("Welcome to {PROJECT_NAME}!").into())
            .text(PROJECT_DESC.into())
            .render();

        println!("{banner}");
    }

    // Parse the command-line arguments
    tracing::trace!(
        token = cli.token,
        verbosity = ?cli.verbosity.as_ref().and_then(|v| v.log_level()),
        command = ?cli.command,
        "Parsed command line arguments"
    );

    // 2. Plugin Initialization Phase
    plugins::broadcast_event(&[], Event::PluginRegistrationInit).await;
    let plugins = plugins::discover_plugins()?;
    for plugin in &plugins {
        plugins::broadcast_event(
            &plugins,
            plugins::Event::PluginRegistered(plugin.manifest.name.clone()),
        )
        .await;
    }
    plugins::broadcast_event(&plugins, Event::PluginRegistrationEnd).await;
    tracing::info!("\n--- Plugin Registration Complete ---\n");

    match &cli.command {
        Commands::Maintain { repo, action } => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let action_arg = action.clone().unwrap_or_else(|| String::from("none"));
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("maintain"),
                args: vec![repo.clone(), action_arg],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            // Run command
            let () = maintain::run(repo.clone(), action)?;

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
        Commands::Merge { repo } => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("merge"),
                args: vec![repo.clone()],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            let () = merge::run(repo.clone())?;

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
        Commands::Wip {
            no_push,
            no_diff,
            rewind,
        } => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("wip"),
                args: vec![
                    no_push.to_string(),
                    no_diff.to_string(),
                    format!("{:#?}", rewind),
                ],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            let _ = wip::run(*no_push, *no_diff, *rewind);

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
        Commands::Prune { yes } => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("prune"),
                args: vec![yes.to_string()],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            match prune::run(*yes) {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e),
            }

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
        Commands::Git { command } => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("git"),
                args: vec![command.to_string()],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            let () = git::run()?;

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
        Commands::Hello {} => {
            // a. Init Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionInit).await;

            // b. Run Event
            let run_event = Event::CliCommandExecutionRun {
                command: String::from("hello"),
                args: vec![],
            };

            plugins::broadcast_event(&plugins, run_event).await;

            let () = hello::run()?;

            // c. End Event
            plugins::broadcast_event(&plugins, Event::CliCommandExecutionEnd).await;
        }
    }

    Ok(())
}
*/
